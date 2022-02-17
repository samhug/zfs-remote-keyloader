mod zfs;

use clap::{Arg, Command};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use tokio::sync::mpsc;

use std::collections::HashMap;
use url::form_urlencoded;

const HTML_WEBFORM: &str = include_str!("../static/index.html");

#[derive(Debug, Clone)]
struct State {
    zfs_dataset: String,
    shutdown_chan: mpsc::Sender<()>,
}

async fn request_handler(req: Request<Body>, state: State) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        // Serve the web form
        (&Method::GET, "/") => Ok(Response::new(Body::from(HTML_WEBFORM))),

        // Load key on POST
        (&Method::POST, "/loadkey") => {
            let body = hyper::body::to_bytes(req.into_body()).await?;

            let params: HashMap<String, String> =
                form_urlencoded::parse(body.as_ref()).into_owned().collect();

            let key = if let Some(k) = params.get("key") {
                k
            } else {
                return Ok(Response::builder()
                    .status(StatusCode::UNPROCESSABLE_ENTITY)
                    .body("Failure: Must provide a key".into())
                    .unwrap());
            };

            match zfs::load_key(&state.zfs_dataset, key) {
                Ok(_) => {
                    state.shutdown_chan.try_send(()).unwrap();
                    Ok(Response::builder()
                        .status(StatusCode::ACCEPTED)
                        .body("Success!".into())
                        .unwrap())
                }
                Err(err) => Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(err.to_string().into())
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
pub async fn main() -> Result<(), hyper::Error> {
    let m = Command::new("zfs-remote-keyloader")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Serves a web form to prompt for ZFS decryption keys")
        .arg(
            Arg::new("addr")
                .short('l')
                .long("listen")
                .value_name("ADDRESS:PORT")
                .help("The IP address and port to listen on")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("zfs-dataset")
                .short('d')
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
    let (tx, mut rx) = mpsc::channel::<()>(1);

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
            rx.recv().await;
        });

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}
