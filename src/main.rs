use std::collections::{HashMap, HashSet};

use clap::{ArgGroup, Parser};
use fast_paths::InputGraph;
use geocoding::openstreetmap::{OpenstreetmapParams, OpenstreetmapResponse};
use geocoding::{InputBounds, Openstreetmap, Point};
use reqwest::blocking::Client;
use serde_derive::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Result, Value};
#[derive(Parser)]
struct Cli {
    address: String,
}

#[derive(Eq, Hash, PartialEq, Debug)]

struct Node {
    name: Option<String>,
    lat: u64,
    lon: u64,
    id: u64,
}
#[derive(Eq, Hash, PartialEq, Debug)]
struct Way {
    id: u64,
    nodes: Vec<Node>,
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

fn response_to_structures(response: Value) -> (Vec<Node>, Vec<Way>) {
    let mut failed = false;
    let mut index = 0;
    let mut amenities: Vec<Node> = Vec::new();
    let mut highways: Vec<Way> = Vec::new();
    while failed == false {
        if response["elements"][index] != json!(null) {
            if response["elements"][index]["tags"]["amenity"] != json!(null) {
                let temp_name;
                let temp_lat;
                let temp_lon;
                let temp_id;
                if response["elements"][index]["type"].to_string() != r##""way""## {
                    temp_lat = response["elements"][index]["lat"]
                        .to_string()
                        .parse::<f64>()
                        .unwrap()
                        .to_bits();
                    temp_lon = response["elements"][index]["lon"]
                        .to_string()
                        .parse::<f64>()
                        .unwrap()
                        .to_bits();
                } else {
                    temp_lat = ((response["elements"][index]["bounds"]["minlat"]
                        .to_string()
                        .parse::<f64>()
                        .unwrap()
                        + response["elements"][index]["bounds"]["maxlat"]
                            .to_string()
                            .parse::<f64>()
                            .unwrap())
                        / 2.0)
                        .to_bits();
                    temp_lon = ((response["elements"][index]["bounds"]["minlon"]
                        .to_string()
                        .parse::<f64>()
                        .unwrap()
                        + response["elements"][index]["bounds"]["maxlon"]
                            .to_string()
                            .parse::<f64>()
                            .unwrap())
                        / 2.0)
                        .to_bits();
                }
                if response["elements"][index]["tags"]["name"] != json!(null) {
                    temp_name = Some(response["elements"][index]["tags"]["name"].to_string());
                } else if response["elements"][index]["tags"]["shop"] != json!(null) {
                    temp_name = Some(response["elements"][index]["tags"]["shop"].to_string());
                } else {
                    temp_name = None;
                }
                if temp_name != None {
                    temp_id = response["elements"][index]["id"].to_string();
                    println!("{:?}", { temp_name.clone() });
                    println!("{:?}", { temp_lat.clone() });
                    println!("{:?}", { temp_lon.clone() });
                    println!("{:?}", { temp_id.clone() });
                    let new_node = Node {
                        name: temp_name,
                        lat: temp_lat,
                        lon: temp_lon,
                        id: temp_id.to_string().parse::<u64>().unwrap(),
                    };
                    amenities.push(new_node);
                }
            } else if response["elements"][index]["tags"]["highway"] != json!(null) {
                let mut way_index = 0;
                let mut failed_way = false;
                let mut nodes_vec: Vec<Node> = Vec::new();
                while failed_way == false {
                    if response["elements"][index]["nodes"][way_index] != json!(null) {
                        let temp_lat;
                        let temp_lon;
                        let temp_id;
                        temp_lat = response["elements"][index]["geometry"][way_index]["lat"]
                            .to_string()
                            .parse::<f64>()
                            .unwrap()
                            .to_bits();
                        temp_lon = response["elements"][index]["geometry"][way_index]["lon"]
                            .to_string()
                            .parse::<f64>()
                            .unwrap()
                            .to_bits();

                        temp_id = response["elements"][index]["nodes"][way_index].to_string();
                        println!("{:?}", { temp_lat.clone() });
                        println!("{:?}", { temp_lon.clone() });
                        println!("{:?}", { temp_id.clone() });
                        let new_node = Node {
                            name: None,
                            lat: temp_lat,
                            lon: temp_lon,
                            id: temp_id.to_string().parse::<u64>().unwrap(),
                        };
                        nodes_vec.push(new_node);
                        way_index += 1;
                    } else {
                        failed_way = true
                    }
                }
                let temp_id = response["elements"][index]["id"].to_string();
                let new_way = Way {
                    id: temp_id.to_string().parse::<u64>().unwrap(),
                    nodes: nodes_vec,
                };
                highways.push(new_way);
            }
            index += 1;
        } else {
            failed = true;
        }
    }
    return (amenities, highways);
}

fn create_graph(highways: Vec<Way>) -> InputGraph {
    let mut input_graph = InputGraph::new();

    return input_graph;
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
    way[highway][highway!=service][highway=footway][access!=private]{bbox};
    way[highway][highway!=service][sidewalk][access!=private]{bbox};
);
out geom;
"##,
        bbox = bounding_box_string
    );
    println!("{}", query);
    let response: Value = get_overpass_json_response(query, coordinates, delta, url);
    // println!("{}", response["version"]);
    let (amenities, highways): (Vec<Node>, Vec<Way>) = response_to_structures(response);
    println!("{:?}", amenities[0]);
    println!("{:?}", highways[0]);
    let path_graph = create_graph(highways);
}
