use clap::{ArgGroup, Parser};
use geocoding::openstreetmap::{OpenstreetmapParams, OpenstreetmapResponse};
use geocoding::{InputBounds, Openstreetmap, Point};
use reqwest::blocking::Client;
use serde_derive::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Result, Value};

#[derive(Serialize, Deserialize, Debug, Parser)]
struct Cli {
    address: String,
}

struct Response {
    elements: [Element; 4096],
}
struct Tags {
    amenity: Option<String>,
    shop: Option<String>,
}
struct Element {
    id: i64,
    r#type: String,
    lat: f64,
    lon: f64,
    tags: Tags,
}

fn main() {
    let args = Cli::parse();
    let delta: f64 = 0.002;
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
    let res = Client::new().post(url).body(query).send();
    // print!("{}", res.unwrap().text().unwrap());
    let response: Value = res.unwrap().json().unwrap();
    // println!("{}", response["version"]);
    let mut failed = false;
    let mut index = 0;
    let mut list: Vec<String> = Vec::new();
    while failed == false {
        if response["elements"][index] != json!(null) {
            if response["elements"][index]["tags"]["name"] != json!(null) {
                list.push(response["elements"][index]["tags"]["name"].to_string());
            } else {
                if response["elements"][index]["tags"]["amenity"] != json!(null) {
                    list.push(response["elements"][index]["tags"]["amenity"].to_string());
                } else {
                    list.push(response["elements"][index]["tags"]["shop"].to_string());
                }
            }
            index += 1;
        } else {
            failed = true;
        }
    }
    println!("{:?}", list);
}
