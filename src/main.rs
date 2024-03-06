mod zfs;

use clap::Parser;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use log::{debug, info, warn};
use tokio::sync::mpsc;

use std::net::SocketAddr;

/// Serves a web form to prompt for ZFS decryption keys
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The IP address and port to listen on
    #[arg(long, value_name = "ADDRESS:PORT")]
    listen_addr: SocketAddr,

    /// The ZFS dataset to load key for
    #[arg(long)]
    zfs_dataset: String,
}

#[derive(Debug, Clone)]
struct State {
    zfs_dataset: String,
    shutdown_signal: mpsc::Sender<()>,
}

async fn request_handler(req: Request<Body>, state: State) -> anyhow::Result<Response<Body>> {
    Ok(match (req.method(), req.uri().path()) {
        // serve the web form
        (&Method::GET, "/") => {
            Response::builder().body(include_str!("../static/index.html").into())?
        }

        // handle form submit
        (&Method::POST, "/loadkey") => {
            let body = hyper::body::to_bytes(req.into_body()).await?;

            let key = {
                let mut params = url::form_urlencoded::parse(&body[..]);
                if let Some(k) = params.find_map(|(k, v)| k.eq("key").then_some(v)) {
                    k
                } else {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body("Invalid request: Must provide a key".into())?);
                }
            };

            if let Err(err) = zfs::load_key(&state.zfs_dataset, key.as_ref()) {
                Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(err.to_string().into())?
            } else {
                info!("Key loaded successfully. Sending shutdown signal now");
                state.shutdown_signal.try_send(())?;
                Response::builder().body("Key loaded successfully".into())?
            }
        }

        // return an error code for everything else
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())?,
    })
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Cli::parse();

    if matches!(
        zfs::get_keystatus(&args.zfs_dataset)?,
        zfs::KeyStatus::Available
    ) {
        warn!(
            "Key for dataset '{}' is already available",
            &args.zfs_dataset
        );
        return Ok(());
    }

    let listen_addr = args.listen_addr;

    // Create a channel for the shutdown signal
    let (shutdown_signal, mut shutdown_signal_rx) = mpsc::channel::<()>(1);

    let state = State {
        zfs_dataset: args.zfs_dataset,
        shutdown_signal,
    };

    let make_svc = {
        use hyper::service::{make_service_fn, service_fn};
        make_service_fn(|_conn| {
            let state = state.clone();
            async {
                let service = service_fn(move |req| request_handler(req, state.clone()));
                Ok::<_, hyper::Error>(service)
            }
        })
    };

    let server = Server::bind(&listen_addr)
        .serve(make_svc)
        .with_graceful_shutdown(async {
            shutdown_signal_rx.recv().await;
            debug!("shutdown signal received");
        });

    info!("Listening on http://{listen_addr}");

    server.await?;

    Ok(())
}
