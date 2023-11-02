use clap::Parser;
use geocoding::openstreetmap::{OpenstreetmapParams, OpenstreetmapResponse};
use geocoding::{InputBounds, Openstreetmap, Point};

use exitfailure::ExitFailure;
use reqwest::Url;
use reqwest::blocking::Client;
use serde::ser::SerializeStructVariant;
use serde_derive::{Deserialize, Serialize};
use std::any::Any;
use std::{env, string};
use urlencoding::encode;
use serde_json::{Result, Value, Map};

#[derive(Serialize, Deserialize, Debug, Parser)]
struct Cli {
    address: String,
}
#[derive(Serialize, Deserialize)]
struct Response {
    elements: String
}
struct Tags {
    amenity: Option<String>,
    shop: Option<String>
}
struct Element {
    id: i64,
    r#type: String,
    lat: f64,
    lon: f64,
    tags: Tags
}

fn main() {
    let args = Cli::parse();
    let delta: f64 = 0.014;
    println!("{}", &args.address);
    let osm = Openstreetmap::new();
    let params = OpenstreetmapParams::new(&args.address)
        .with_addressdetails(true)
        .build();
    let res: OpenstreetmapResponse<f64> = osm.forward_full(&params).unwrap();
    let coordinates = res.features[0].geometry.coordinates;
    let bounding_box = (
        (coordinates.1 - delta),
        (coordinates.0 - delta),
        (coordinates.1 + delta),
        (coordinates.0 + delta),
    );
    let bounding_box_string = format!(
        "({},{},{},{})",
        bounding_box.0, bounding_box.1, bounding_box.2, bounding_box.3
    );
    let url = "https://maps.mail.ru/osm/tools/overpass/api/interpreter";
    let query = format!(
r##"[out:json]
[timeout:25];
(
    nwr["amenity"]{};
    nwr["shop"]{};
);
out geom;"##,
    bounding_box_string, bounding_box_string
    );
    println!("{}", query);
    let res = Client::new()
        .post(url)
        .body(query)
        .send();
    let something = res.unwrap().text().unwrap();
    let something = serde_json::from_str(&something);
    // match res {
    //     Ok(response) => {
    //         println!("{}", response.status());
    // },
    //     Err(err) => todo!(),
    //   }
    println!("done")
}
