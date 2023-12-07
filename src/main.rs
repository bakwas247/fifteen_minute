use core::f64;
use std::collections::{HashMap, HashSet};

use std::usize;

use bimap::{BiHashMap, BiMap};
use clap::Parser;
use fast_paths::InputGraph;
use geocoding::openstreetmap::{OpenstreetmapParams, OpenstreetmapResponse};
use geocoding::Openstreetmap;
use haversine_redux::Location;
use kiddo::{ImmutableKdTree, NearestNeighbour, SquaredEuclidean};
use rayon::prelude::*;
use reqwest::blocking::Client;
use serde_json::json;
use serde_json::Value;

#[derive(Parser)]
struct Cli {
    address: String,
    distance: u64,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]

struct Node {
    name: Option<String>,
    coordinate: (u64, u64),
    id: usize,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
struct Way {
    id: usize,
    nodes: Vec<Node>,
}

fn get_overpass_json_response(
    coordinates: (f64, f64),
    deltay: f64,
    deltax: f64,
    url: String,
) -> Value {
    let bounding_box = (
        (coordinates.1 - deltax),
        (coordinates.0 - deltay * 2.0),
        (coordinates.1 + deltax),
        (coordinates.0 + deltay * 2.0),
    );
    let bounding_box_string = format!(
        "({},{},{},{})",
        bounding_box.0, bounding_box.1, bounding_box.2, bounding_box.3
    );
    let query = format!(
        r##"
[out:json]
[timeout:60];
(
    nwr["amenity"][type!=relation][type!=multipolygon]{bbox};
    nwr["shop"][type!=relation][type!=multipolygon]{bbox};
    way[highway][highway!=service][highway=footway][access!=private][type!=relation][type!=multipolygon]{bbox};
    way[highway][highway!=service][sidewalk][access!=private][type!=relation][type!=multipolygon]{bbox};
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
    let urls: Vec<String> = vec![
        "https://maps.mail.ru/osm/tools/overpass/api/interpreter".to_string(),
        "https://overpass-api.de/api/interpreter".to_string(),
        "https://overpass.kumi.systems/api/interpreter".to_string(),
    ];
    let index = 0;
    let mut url = &urls[index];
    let mut valid_url = false;
    let query = format!(
        r##"
    [out:json]
    [timeout:25];
    (
    nwr["amenity"](55.947049399999995,-3.1827352999999997,55.9490494,-3.1817353);
    );
    out geom;
            "##
    );
    while valid_url != true && index < urls.len() {
        url = &urls[index];
        let res = Client::new().post(url).body(query.clone()).send();
        if res.unwrap().status().is_success() {
            valid_url = true;
        }
    }
    return url.clone();
}

fn response_to_structures(
    response: Value,
) -> (
    Vec<Node>,
    Vec<Way>,
    HashMap<usize, Node>,
    BiHashMap<usize, usize>,
) {
    let mut failed = false;
    let mut index = 0;
    let mut amenities: Vec<Node> = Vec::new();
    let mut highway_nodes: HashMap<usize, Node> = HashMap::new();
    let mut highways: Vec<Way> = Vec::new();
    while failed == false {
        if response["elements"][index] != json!(null) {
            if response["elements"][index]["tags"]["amenity"] != json!(null) {
                let temp_name;
                let temp_lat;
                let temp_lon;
                let temp_id;
                println!("{:?}", response["elements"][index]);
                if response["elements"][index]["type"].to_string() == r##""node""## {
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
                    let new_node = Node {
                        name: temp_name,
                        coordinate: (temp_lat, temp_lon),
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
                        let new_node = Node {
                            name: None,
                            coordinate: (temp_lat, temp_lon),
                            id: temp_id.to_string().parse::<usize>().unwrap(),
                        };
                        nodes_vec.push(new_node.clone());
                        highway_nodes.insert(new_node.id, new_node);
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
    for value in highway_nodes.iter() {
        nodes_lookup_table.insert(adder_index, value.1.id);
        adder_index += 1;
    }
    for node in amenities.iter() {
        nodes_lookup_table.insert(adder_index, node.id);
        adder_index += 1;
    }
    return (amenities, highways, highway_nodes, nodes_lookup_table);
}

// fn get_node_id(graph_id: usize, node_lut: &BiHashMap<usize, usize>) -> usize {
//     let res: usize = node_lut
//         .get_by_left(&graph_id)
//         .unwrap_or(&usize::MAX)
//         .clone();
//     return res;
// }

// fn get_node_from_coord(coord: (u64, u64), nodes: &HashSet<Node>) -> Node {
//     let res: Node = nodes
//         .get(&coord)
//         .unwrap_or(&Node {
//             name: None,
//             coordinate: (u64::MAX, u64::MAX),
//             id: (usize::MAX),
//         })
//         .clone();
//     return res;
// }

// fn get_node_from_id(id: usize, nodes: &HashSet<Node>) -> Node {
//     let res: Node = nodes
//         .get(&id)
//         .unwrap_or(&Node {
//             name: None,
//             coordinate: (u64::MAX, u64::MAX),
//             id: (usize::MAX),
//         })
//         .clone();
//     return res;
// }

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
    highway_nodes: HashMap<usize, Node>,
    node_lut: BiHashMap<usize, usize>,
    neighbour_nodes: ImmutableKdTree<f64, 2>,
    entries: Vec<usize>,
) -> InputGraph {
    let mut total_nodes: HashMap<usize, Node> = highway_nodes.clone();
    for node in amenities.iter() {
        total_nodes.insert(node.id, node.clone());
    }
    let mut input_graph = InputGraph::new();
    let road_edges: Vec<Vec<(usize, usize, usize)>> = highways
        .par_iter()
        .map(|highway| {
            let mut last_node: Node = Node {
                coordinate: (0, 0),
                id: 0,
                name: Some("Uninitialised".to_string()),
            };
            let mut edges: Vec<(usize, usize, usize)> = Vec::new();
            for node in highway.nodes.iter() {
                if last_node.name != Some("Uninitialised".to_string()) {
                    let node1 = get_graph_id(node.id, &node_lut);
                    let node2 = get_graph_id(last_node.id, &node_lut);
                    let start: Location = Location::new(
                        f64::from_bits(node.coordinate.0),
                        f64::from_bits(node.coordinate.1),
                    );
                    let end: Location = Location::new(
                        f64::from_bits(last_node.coordinate.0),
                        f64::from_bits(last_node.coordinate.1),
                    );
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
            let nearest: NearestNeighbour<f64, u64> = neighbour_nodes
                .nearest_one::<SquaredEuclidean>(&[
                    f64::from_bits(node.coordinate.0),
                    f64::from_bits(node.coordinate.1),
                ]);
            let index: u64 = nearest.item;
            println!("{:?}", entries[index as usize]);
            let neighbour_node = &highway_nodes[&entries[index as usize]];
            println!("{:?}", neighbour_node);
            let start: Location = Location::new(
                f64::from_bits(node.coordinate.0),
                f64::from_bits(node.coordinate.1),
            );
            let end: Location = Location::new(
                f64::from_bits(neighbour_node.coordinate.0),
                f64::from_bits(neighbour_node.coordinate.1),
            );
            return (
                get_graph_id(node.id, &node_lut),
                get_graph_id(entries[index as usize], &node_lut) as usize,
                (start.kilometers_to(&end) * 1000.0) as usize,
            );
        })
        .collect();
    let mut edges = road_edges.concat();
    edges.append(&mut neighbour_edges);
    println!("{:?}", edges);
    for edge in edges.iter() {
        input_graph.add_edge_bidir(edge.0, edge.1, edge.2);
    }

    input_graph.freeze();
    let _edge_count = input_graph.get_num_edges();
    return input_graph;
}

fn create_kdtree(highway_nodes: HashMap<usize, Node>) -> (ImmutableKdTree<f64, 2>, Vec<usize>) {
    let mut entries: Vec<[f64; 2]> = Vec::new();
    let mut entries_id: Vec<usize> = Vec::new();

    for value in highway_nodes.iter() {
        entries.push([
            f64::from_bits(value.1.coordinate.0),
            f64::from_bits(value.1.coordinate.1),
        ]);
        entries_id.push(value.1.id);
    }
    let tree: ImmutableKdTree<f64, 2> = ImmutableKdTree::new_from_slice(&entries);
    return (tree, entries_id);
}

fn cull_amenities(
    amenities: Vec<Node>,
    path_graph: InputGraph,
    nearest_node: u64,
    node_lut: BiHashMap<usize, usize>,
    distance: u64,
) -> Vec<Node> {
    let mut amenity_hashset: HashSet<Node> = HashSet::new();
    for amenity in amenities.iter() {
        amenity_hashset.insert(amenity.clone());
    }
    let fast_graph = fast_paths::prepare(&path_graph);

    // calculate the shortest path between nodes with ID 8 and 6

    let new_amenity_list: Vec<Node> = amenity_hashset
        .iter()
        .map(|node: &Node| {
            let shortest_path = fast_paths::calc_path(
                &fast_graph,
                nearest_node as usize,
                get_graph_id(node.id, &node_lut),
            );
            let mut safe = 0;

            match shortest_path {
                Some(p) => {
                    // the weight of the shortest path
                    let weight = p.get_weight();

                    // all nodes of the shortest path (including source and target)
                    // let nodes = p.get_nodes();

                    if weight < distance as usize {
                        safe = 1;
                    }
                }
                None => {
                    safe = 0;
                }
            }
            if safe == 1 {
                return node.clone();
            } else {
                return node.clone();
            }
        })
        .collect();

    return new_amenity_list;
}

fn main() {
    let args = Cli::parse();
    let deltay: f64 = (args.distance as f64 / 111000.0).abs();
    let url = get_active_url();
    println!("{}", &args.address);
    let coordinates = get_address_coordinates(args.address);
    let deltax: f64 = (deltay / coordinates.1.cos()).abs();
    let response: Value = get_overpass_json_response(coordinates, deltay, deltax, url);
    // println!("{}", response["version"]);
    let (amenities, highways, highway_nodes, nodes_lut): (
        Vec<Node>,
        Vec<Way>,
        HashMap<usize, Node>,
        BiHashMap<usize, usize>,
    ) = response_to_structures(response);
    println!("{:?}", amenities[0]);
    println!("{:?}", highways[0]);
    let (search_tree, entries): (ImmutableKdTree<f64, 2>, Vec<usize>) =
        create_kdtree(highway_nodes.clone());
    let nearest: NearestNeighbour<f64, u64> =
        search_tree.nearest_one::<SquaredEuclidean>(&[coordinates.0, coordinates.1]);
    let path_graph = create_graph(
        amenities.clone(),
        highways,
        highway_nodes,
        nodes_lut.clone(),
        search_tree,
        entries,
    );
    let new_amenities = cull_amenities(
        amenities,
        path_graph,
        nearest.item,
        nodes_lut,
        args.distance,
    );
    for amenity in new_amenities.iter() {
        println!("{:?}", amenity);
    }
}
