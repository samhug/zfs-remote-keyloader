extern crate clap;
extern crate futures;
extern crate hyper;
extern crate url;

use clap::{App, Arg};

use futures::future;
use futures::sync::mpsc;
use hyper::rt::{Future, Stream};
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, Server, StatusCode};

use std::collections::HashMap;
use url::form_urlencoded;

#[derive(Debug)]
struct LoadKeyErr {
    message: String,
}

impl LoadKeyErr {
    fn new(msg: String) -> Self {
        return LoadKeyErr { message: msg };
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

type BoxFut = Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send>;

fn request_handler(req: Request<Body>, mut state: State) -> BoxFut {
    match (req.method(), req.uri().path()) {
        // Serve the web form
        (&Method::GET, "/") => Box::new(future::ok(Response::new(Body::from(HTML_WEBFORM)))),

        // Load key on POST
        (&Method::POST, "/loadkey") => Box::new(req.into_body().concat2().map(move |body| {
            let params = form_urlencoded::parse(body.as_ref())
                .into_owned()
                .collect::<HashMap<String, String>>();

            let key = if let Some(k) = params.get("key") {
                k
            } else {
                return Response::builder()
                    .status(StatusCode::UNPROCESSABLE_ENTITY)
                    .body("Failure: Must provide a key".into())
                    .unwrap();
            };

            match zfs_loadkey(state.zfs_dataset, key.to_string()) {
                Ok(_) => {
                    state.shutdown_chan.try_send(()).unwrap();
                    Response::builder()
                        .status(StatusCode::ACCEPTED)
                        .body("Success!".into())
                        .unwrap()
                }
                Err(err) => Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(err.message.into())
                    .unwrap(),
            }
        })),

        // Return an error code for everything else
        _ => Box::new(future::ok(
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap(),
        )),
    }
}

#[derive(Debug, Clone)]
struct State {
    zfs_dataset: String,
    shutdown_chan: mpsc::Sender<()>,
}

fn main() {
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
        zfs_dataset: zfs_dataset,
        shutdown_chan: tx,
    };

    // TODO: I don't understand what this does
    let make_service = move || {
        let state2 = state.clone();
        service_fn(move |req| request_handler(req, state2.clone()))
    };

    let server = Server::bind(&addr)
        .serve(make_service)
        .with_graceful_shutdown(rx.into_future().map(|_| ()).map_err(|_| ()))
        .map_err(|_| {
            eprintln!("server shutdown");
        });

    println!("Listening on http://{}", addr);
    hyper::rt::run(server);
}
