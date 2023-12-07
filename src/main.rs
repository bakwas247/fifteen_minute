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

fn get_overpass_json_response(
    query: String,
    coordinates: (f64, f64),
    delta: f64,
    url: String,
) -> Value {
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
    let res = Client::new().post(url).body(query).send();
    let response: Value = res.unwrap().json().unwrap();
    return response;
}

fn get_address_coordinates(address: String) -> (f64, f64) {
    let osm = Openstreetmap::new();
    let params = OpenstreetmapParams::new(&address)
        .with_addressdetails(true)
        .build();
    let res: OpenstreetmapResponse<f64> = osm.forward_full(&params).unwrap();
    let coordinates = res.features[0].geometry.coordinates;
    return coordinates;
}

fn get_active_url() -> String {
    let url: String = "https://maps.mail.ru/osm/tools/overpass/api/interpreter".to_owned();
    return url;
}

fn response_to_structures(response: Value) -> Vec<String> {
    let mut failed = false;
    let mut index = 0;
    let mut amenities: Vec<String> = Vec::new();
    while failed == false {
        if response["elements"][index] != json!(null) {
            if response["elements"][index]["tags"]["name"] != json!(null) {
                amenities.push(response["elements"][index]["tags"]["name"].to_string());
            } else {
                if response["elements"][index]["tags"]["amenity"] != json!(null) {
                    // list.push(response["elements"][index]["tags"]["amenity"].to_string());
                } else {
                    amenities.push(response["elements"][index]["tags"]["shop"].to_string());
                }
            }
            index += 1;
        } else {
            failed = true;
        }
    }
    return amenities;
}

fn main() {
    let args = Cli::parse();
    let delta: f64 = 0.002;
    let url = get_active_url();
    println!("{}", &args.address);
    let coordinates = get_address_coordinates(args.address);
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
    let query = format!(
        r##"
[out:json]
[timeout:25];
(
    nwr["amenity"]{bbox};
    nwr["shop"]{bbox};
);
out geom;
"##,
        bbox = bounding_box_string
    );
    println!("{}", query);
    let response: Value = get_overpass_json_response(query, coordinates, delta, url);
    // println!("{}", response["version"]);
    let amenities: Vec<String> = response_to_structures(response);
    println!("{:?}", amenities);
}
