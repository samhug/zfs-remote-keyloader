#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate clap;

use clap::{App, Arg};
use rocket::fairing::AdHoc;
use rocket::request::Form;
use rocket::response::content::Html;

// == Default Route ==
// Serve up a web form prompting for a decryption key
#[get("/")]
fn index() -> Html<&'static str> {
    Html(
        r#"<!doctype html>
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
</html>"#,
    )
}

#[derive(FromForm)]
struct KeyForm {
    key: String,
}

#[post("/loadkey", data = "<key>")]
fn loadkey(key: Form<KeyForm>) -> String {
    use std::io::Write;
    use std::process::Command;
    use tempfile::NamedTempFile;

    let dataset = "rpool";

    // create a temp file to hold the key
    // TODO: You really shouldn't write the key to disk...
    let mut key_file = NamedTempFile::new().expect("failed to create temporary key file");

    // write the key to the temp file
    write!(key_file, "{}", key.key).expect("failed to write key to temporary key file");

    let key_file_path = key_file.into_temp_path();

    let status = Command::new("zfs")
        .arg("load-key")
        .arg("-L")
        .arg(format!("file://{}", key_file_path.to_str().unwrap()))
        .arg(dataset)
        .status()
        .expect("failed to execute zfs load-key command");

    if !status.success() {
        return match status.code() {
            Some(code) => format!("Failed to load key: exit-code {}", code),
            None => format!("Failed to load key: Process terminated by signal"),
        };
    }

    key_file_path
        .close()
        .expect("failed to clean up temporary key file");

    format!("Success!")
}

struct DatasetName(String);

fn main() {
    let m = App::new("zfs-remote-keyloader")
        .version("v0.0.1")
        .about("Serves a web form to prompt for ZFS decryption keys")
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

    let zfs_dataset = String::from(m.value_of("zfs-dataset").unwrap());

    rocket::ignite()
        .mount("/", routes![index, loadkey])
        .attach(AdHoc::on_attach("ZFS Dataset Name", |rocket| {
            Ok(rocket.manage(DatasetName(zfs_dataset)))
        }))
        .launch();
}
