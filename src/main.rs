extern crate clap;
extern crate futures;
extern crate hyper;
extern crate url;

use clap::{App, Arg};

use futures::StreamExt;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use tokio::sync::mpsc;

use std::collections::HashMap;
use url::form_urlencoded;

#[derive(Debug)]
struct LoadKeyErr {
    message: String,
}

impl LoadKeyErr {
    fn new(msg: String) -> Self {
        LoadKeyErr { message: msg }
    }
}

fn zfs_loadkey(dataset: String, key: String) -> Result<(), LoadKeyErr> {
    use std::io::Write;
    use std::process::Command;
    use tempfile::NamedTempFile;

    // create a temp file to hold the key
    // TODO: You really shouldn't write the key to disk...
    let mut key_file = NamedTempFile::new()
        .map_err(|_e| LoadKeyErr::new("failed to create temporary key file".to_string()))?;

    // write the key to the temp file
    write!(key_file, "{}", key)
        .map_err(|_e| LoadKeyErr::new("failed to write key to temporary key file".to_string()))?;

    let key_file_path = key_file.into_temp_path();

    let cmd = Command::new("zfs")
        .arg("load-key")
        .arg("-L")
        .arg(format!("file://{}", key_file_path.to_str().unwrap()))
        .arg(dataset)
        .output()
        .map_err(|e| LoadKeyErr::new(format!("zfs load-key failed: {}", e)))?;

    if !cmd.status.success() {
        let output = std::str::from_utf8(&cmd.stderr).unwrap_or("<invalid UTF-8 output>");
        return Err(LoadKeyErr::new(format!("Failed to load key: {}", output)));
    }

    key_file_path
        .close()
        .map_err(|_e| LoadKeyErr::new("failed to clean up temporary key file".to_string()))?;

    Ok(())
}

const HTML_WEBFORM: &[u8] = br#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>ZFS Remote Keyloader</title>
  <meta name="description" content="ZFS Remote Keyloader">
  <link rel="stylesheet" href="css/styles.css?v=1.0">
</head>

<body>
  <p>Hello!</p>
  <form action="/loadkey" method="post">
    <input type="password" name="key" autofocus>
    <input type="submit" value="Load Key">
  </form>
</body>
</html>"#;

#[derive(Debug, Clone)]
struct State {
    zfs_dataset: String,
    shutdown_chan: mpsc::Sender<()>,
}

async fn request_handler(
    req: Request<Body>,
    mut state: State,
) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        // Serve the web form
        (&Method::GET, "/") => Ok(Response::new(Body::from(HTML_WEBFORM))),

        // Load key on POST
        (&Method::POST, "/loadkey") => {
            let body = hyper::body::to_bytes(req.into_body()).await?;

            let params = form_urlencoded::parse(body.as_ref())
                .into_owned()
                .collect::<HashMap<String, String>>();

            let key = if let Some(k) = params.get("key") {
                k
            } else {
                return Ok(Response::builder()
                    .status(StatusCode::UNPROCESSABLE_ENTITY)
                    .body("Failure: Must provide a key".into())
                    .unwrap());
            };

            match zfs_loadkey(state.zfs_dataset, key.to_string()) {
                Ok(_) => {
                    state.shutdown_chan.try_send(()).unwrap();
                    Ok(Response::builder()
                        .status(StatusCode::ACCEPTED)
                        .body("Success!".into())
                        .unwrap())
                }
                Err(err) => Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(err.message.into())
                    .unwrap()),
            }
        }

        // Return an error code for everything else
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let m = App::new("zfs-remote-keyloader")
        .version("v0.1.0")
        .about("Serves a web form to prompt for ZFS decryption keys")
        .arg(
            Arg::with_name("addr")
                .short("l")
                .long("listen")
                .value_name("ADDRESS:PORT")
                .help("The IP address and port to listen on")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("zfs-dataset")
                .short("d")
                .long("zfs-dataset")
                .value_name("DATASET")
                .help("The ZFS dataset to load key for")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let addr = m.value_of("addr").unwrap().parse().unwrap();
    let zfs_dataset = String::from(m.value_of("zfs-dataset").unwrap());

    // Create a channel for the shutdown signal
    let (tx, rx) = mpsc::channel::<()>(1);

    let state = State {
        zfs_dataset,
        shutdown_chan: tx,
    };

    let make_svc = make_service_fn(|_conn| {
        let state = state.clone();
        async {
            let service = service_fn(move |req| request_handler(req, state.clone()));
            Ok::<_, hyper::Error>(service)
        }
    });

    let server = Server::bind(&addr)
        .serve(make_svc)
        .with_graceful_shutdown(async {
            rx.into_future().await;
        });

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}
