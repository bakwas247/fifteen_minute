use std::borrow::Borrow;
use std::collections::HashSet;
use std::ptr::null;
use std::{path, usize};

use bimap::{BiHashMap, BiMap};
use clap::Parser;
use fast_paths::InputGraph;
use geocoding::openstreetmap::{OpenstreetmapParams, OpenstreetmapResponse};
use geocoding::Openstreetmap;
use haversine_redux::{Location, Unit};
use kiddo::{ImmutableKdTree, SquaredEuclidean};
use rayon::prelude::*;
use reqwest::blocking::Client;
use serde_json::json;
use serde_json::Value;
#[derive(Parser)]
struct Cli {
    address: String,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]

struct Node {
    name: Option<String>,
    lat: u64,
    lon: u64,
    id: usize,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
struct GraphNode {
    coordinate: (u64, u64),
    id: u64,
    graph_id: usize,
}

impl Borrow<(u64, u64)> for GraphNode {
    fn borrow(&self) -> &(u64, u64) {
        &self.coordinate
    }
}
impl Borrow<usize> for GraphNode {
    fn borrow(&self) -> &usize {
        &self.graph_id
    }
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
struct Way {
    id: usize,
    nodes: Vec<Node>,
}

fn get_overpass_json_response(coordinates: (f64, f64), delta: f64, url: String) -> Value {
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

fn response_to_structures(
    response: Value,
) -> (Vec<Node>, Vec<Way>, HashSet<Node>, BiHashMap<usize, usize>) {
    let mut failed = false;
    let mut index = 0;
    let mut amenities: Vec<Node> = Vec::new();
    let mut highway_nodes: HashSet<Node> = HashSet::new();
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
                        id: temp_id.to_string().parse::<usize>().unwrap(),
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
                            id: temp_id.to_string().parse::<usize>().unwrap(),
                        };
                        nodes_vec.push(new_node.clone());
                        highway_nodes.insert(new_node);
                        way_index += 1;
                    } else {
                        failed_way = true
                    }
                }
                let temp_id = response["elements"][index]["id"].to_string();
                let new_way = Way {
                    id: temp_id.to_string().parse::<usize>().unwrap(),
                    nodes: nodes_vec,
                };
                highways.push(new_way);
            }
            index += 1;
        } else {
            failed = true;
        }
    }
    let mut adder_index: usize = 0;
    let mut nodes_lookup_table: BiHashMap<usize, usize> = BiMap::new();
    for node in highway_nodes.iter() {
        nodes_lookup_table.insert(adder_index, node.id);
        adder_index += 1;
    }
    for node in amenities.iter() {
        nodes_lookup_table.insert(adder_index, node.id);
        adder_index += 1;
    }
    return (amenities, highways, highway_nodes, nodes_lookup_table);
}

fn get_node_id(graph_id: usize, node_lut: &BiHashMap<usize, usize>) -> usize {
    let res: usize = node_lut
        .get_by_left(&graph_id)
        .unwrap_or(&usize::MAX)
        .clone();
    return res;
}

fn get_graph_id(node_id: usize, node_lut: &BiHashMap<usize, usize>) -> usize {
    let res = node_lut
        .get_by_right(&node_id)
        .unwrap_or(&usize::MAX)
        .clone();
    return res;
}

fn create_graph(
    amenities: Vec<Node>,
    highways: Vec<Way>,
    highway_nodes: HashSet<Node>,
    node_lut: BiHashMap<usize, usize>,
    neighbour_nodes: ImmutableKdTree<f64, 2>,
    entries: Vec<[f64; 2]>,
) -> InputGraph {
    let mut total_nodes: HashSet<Node> = highway_nodes.clone();
    for node in amenities.iter() {
        total_nodes.insert(node.clone());
    }
    let mut input_graph = InputGraph::new();
    let road_edges: Vec<Vec<(usize, usize, usize)>> = highways
        .par_iter()
        .map(|highway| {
            let mut last_node: Node = Node {
                lat: 0,
                lon: 0,
                id: 0,
                name: Some("Uninitialised".to_string()),
            };
            let mut edges: Vec<(usize, usize, usize)> = Vec::new();
            for node in highway.nodes.iter() {
                if last_node.name != Some("Uninitialised".to_string()) {
                    let node1 = get_graph_id(node.id, &node_lut);
                    let node2 = get_graph_id(last_node.id, &node_lut);
                    let start: Location =
                        Location::new(f64::from_bits(node.lat), f64::from_bits(node.lon));
                    let end: Location =
                        Location::new(f64::from_bits(last_node.lat), f64::from_bits(last_node.lon));
                    let something = (node1, node2, (start.kilometers_to(&end) * 1000.0) as usize);
                    edges.push(something);
                }
                last_node = node.clone();
            }
            return edges;
        })
        .collect();
    // let mut edges: Vec<(usize, usize, usize)> = Vec::new();
    // for edge_vec in road_edges.iter() {
    //     edges.concat()
    // }
    let mut neighbour_edges: Vec<(usize, usize, usize)> = amenities
        .par_iter()
        .map(|node: &Node| {
            let nearest = neighbour_nodes.nearest_one::<SquaredEuclidean>(&[
                f64::from_bits(node.lat),
                f64::from_bits(node.lon),
            ]);
            println!("{:?}", nearest);
            return (0 as usize, 0 as usize, 0 as usize);
        })
        .collect();
    let mut edges = road_edges.concat();
    edges.append(&mut neighbour_edges);
    let _ = edges
        .iter()
        .map(|edge| input_graph.add_edge_bidir(edge.0, edge.1, edge.2));

    return input_graph;
}

fn create_kdtree(highway_nodes: HashSet<Node>) -> (ImmutableKdTree<f64, 2>, Vec<[f64; 2]>) {
    let entries: Vec<[f64; 2]> = highway_nodes
        .par_iter()
        .map(|node| [f64::from_bits(node.lat), f64::from_bits(node.lon)])
        .collect();
    let tree: ImmutableKdTree<f64, 2> = ImmutableKdTree::new_from_slice(&entries);
    return (tree, entries);
}

fn main() {
    let args = Cli::parse();
    let delta: f64 = 0.002;
    let url = get_active_url();
    println!("{}", &args.address);
    let coordinates = get_address_coordinates(args.address);
    let response: Value = get_overpass_json_response(coordinates, delta, url);
    // println!("{}", response["version"]);
    let (amenities, highways, highway_nodes, nodes_lut): (
        Vec<Node>,
        Vec<Way>,
        HashSet<Node>,
        BiHashMap<usize, usize>,
    ) = response_to_structures(response);
    println!("{:?}", amenities[0]);
    println!("{:?}", highways[0]);
    let (search_tree, entries): (ImmutableKdTree<f64, 2>, Vec<[f64; 2]>) =
        create_kdtree(highway_nodes.clone());
    let path_graph = create_graph(
        amenities,
        highways,
        highway_nodes,
        nodes_lut,
        search_tree,
        entries,
    );
}
