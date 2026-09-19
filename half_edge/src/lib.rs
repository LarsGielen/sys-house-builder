use std::{collections::HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct WallNodeId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EdgeId(usize);

#[derive(Clone, Copy)]
struct WallNode {
	x: f32,
	y: f32,
	edge_id: Option<EdgeId>,
}

struct HalfEdge {
	origin: WallNodeId,
	twin: EdgeId,
	next: EdgeId,
	previous: EdgeId,
}

struct SpatialGraph {
	next_wall_node_id: usize,
	next_edge_id: usize,
	vertices: HashMap<WallNodeId, WallNode>,
	edges: HashMap<EdgeId, HalfEdge>,
}

impl SpatialGraph {
	fn new() -> Self {
		SpatialGraph {
			next_wall_node_id: 0,
			next_edge_id: 0,
			vertices: HashMap::new(),
			edges: HashMap::new(),
		}
	}

	fn get_next_wall_node_id(&mut self) -> WallNodeId {
		let id = self.next_wall_node_id;
		self.next_wall_node_id += 1;

		WallNodeId(id)
	}

	fn get_next_edge_id(&mut self) -> EdgeId {
		let id = self.next_edge_id;
		self.next_edge_id += 1;

		EdgeId(id)
	}

	fn add_vertex(&mut self, x: f32, y: f32) -> WallNodeId {
		let id = self.get_next_wall_node_id();
		self.vertices.insert(id, WallNode { x, y, edge_id: Option::None });
		id
	}

	fn add_edge(&mut self, origin_id: WallNodeId, destination_id: WallNodeId) {
		let edge_id_1 = self.get_next_edge_id();
		let edge_id_2 = self.get_next_edge_id();

		let origin = self.vertices[&origin_id];
		let destination = self.vertices[&destination_id];

		let edge1 = HalfEdge { origin: origin_id,		twin: edge_id_2,	next: destination.edge_id.unwrap_or(edge_id_2),	previous: origin.edge_id.unwrap_or(edge_id_2) };
		let edge2 = HalfEdge { origin: destination_id, 	twin: edge_id_1, 	next: origin.edge_id.unwrap_or(edge_id_1), 		previous: destination.edge_id.unwrap_or(edge_id_1) };
		self.edges.insert(edge_id_1, edge1);
		self.edges.insert(edge_id_2, edge2);

		if let Some(edge_id) = origin.edge_id {
			let twin_id = self.edges[&edge_id].twin;
			if let Some(edge) = self.edges.get_mut(&edge_id_1) { edge.previous = twin_id }
			if let Some(edge) = self.edges.get_mut(&twin_id) { edge.next = edge_id_1 }
			if let Some(edge) = self.edges.get_mut(&edge_id) { edge.previous = edge_id_2 }
		}

		if let Some(edge_id) = destination.edge_id {
			let twin_id = self.edges[&edge_id].twin;
			if let Some(edge) = self.edges.get_mut(&edge_id_2) { edge.previous = twin_id }
			if let Some(edge) = self.edges.get_mut(&twin_id) { edge.next = edge_id_2 }
			if let Some(edge) = self.edges.get_mut(&edge_id) { edge.previous = edge_id_1 }
		}

		if let Some(node) = self.vertices.get_mut(&origin_id) { node.edge_id = Some(edge_id_1); }
		if let Some(node) = self.vertices.get_mut(&destination_id) { node.edge_id = Some(edge_id_2); }

	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn destination(graph: &SpatialGraph, edge: EdgeId) -> WallNodeId {
		graph.edges[&graph.edges[&edge].twin].origin
	}

	fn find_edge(graph: &SpatialGraph, from: WallNodeId, to: WallNodeId) -> EdgeId {
		*graph
			.edges
			.keys()
			.find(|&&id| graph.edges[&id].origin == from && destination(graph, id) == to)
			.expect("no half-edge found between the given nodes")
	}

	fn assert_consistent(graph: &SpatialGraph) {
		for (&id, edge) in &graph.edges {
			assert_eq!(graph.edges[&edge.twin].twin, id, "twin of twin must be self");
			assert_eq!(graph.edges[&edge.next].previous, id, "previous(next(e)) must be e");
			assert_eq!(graph.edges[&edge.previous].next, id, "next(previous(e)) must be e");
			assert_eq!(
				graph.edges[&edge.next].origin,
				destination(graph, id),
				"next(e) must start where e ends"
			);
			assert_eq!(
				destination(graph, edge.previous),
				edge.origin,
				"previous(e) must end where e starts"
			);
		}
		for (&node_id, node) in &graph.vertices {
			if let Some(edge_id) = node.edge_id {
				assert_eq!(graph.edges[&edge_id].origin, node_id, "node.edge must start at that node");
			}
		}
	}

	#[test]
	fn two_walls_sharing_a_node_form_one_four_edge_loop() {
		let mut graph = SpatialGraph::new();
		let a = graph.add_vertex(0.0, 0.0);
		let b = graph.add_vertex(1.0, 0.0);
		let c = graph.add_vertex(1.0, 1.0);
		graph.add_edge(a, b);
		graph.add_edge(b, c);

		assert_consistent(&graph);

		let a_to_b = find_edge(&graph, a, b);
		let b_to_a = find_edge(&graph, b, a);
		let b_to_c = find_edge(&graph, b, c);
		let c_to_b = find_edge(&graph, c, b);

		// Walking the boundary: A->B, turn onto B->C, tip at C, back C->B, turn onto B->A, tip at A.
		assert_eq!(graph.edges[&a_to_b].next, b_to_c);
		assert_eq!(graph.edges[&b_to_c].next, c_to_b);
		assert_eq!(graph.edges[&c_to_b].next, b_to_a);
		assert_eq!(graph.edges[&b_to_a].next, a_to_b);
	}

	#[test]
	fn joining_two_existing_walls_forms_one_six_edge_loop() {
		let mut graph = SpatialGraph::new();
		let a = graph.add_vertex(0.0, 0.0);
		let b = graph.add_vertex(1.0, 0.0);
		let c = graph.add_vertex(2.0, 0.0);
		let d = graph.add_vertex(3.0, 0.0);
		graph.add_edge(a, b);
		graph.add_edge(c, d);
		graph.add_edge(b, c);

		assert_consistent(&graph);

		let a_to_b = find_edge(&graph, a, b);
		let b_to_a = find_edge(&graph, b, a);
		let b_to_c = find_edge(&graph, b, c);
		let c_to_b = find_edge(&graph, c, b);
		let c_to_d = find_edge(&graph, c, d);
		let d_to_c = find_edge(&graph, d, c);

		// Walking the boundary of the path A-B-C-D: out along one side, tip at D, back along the other, tip at A.
		assert_eq!(graph.edges[&a_to_b].next, b_to_c);
		assert_eq!(graph.edges[&b_to_c].next, c_to_d);
		assert_eq!(graph.edges[&c_to_d].next, d_to_c);
		assert_eq!(graph.edges[&d_to_c].next, c_to_b);
		assert_eq!(graph.edges[&c_to_b].next, b_to_a);
		assert_eq!(graph.edges[&b_to_a].next, a_to_b);
	}

	#[test]
	fn isolated_wall_links_form_a_two_edge_loop() {
		let mut graph = SpatialGraph::new();
		let a = graph.add_vertex(0.0, 0.0);
		let b = graph.add_vertex(1.0, 0.0);
		graph.add_edge(a, b);

		let e1 = graph.vertices[&a].edge_id.expect("origin node should have an outgoing edge");
		let e2 = graph.vertices[&b].edge_id.expect("destination node should have an outgoing edge");
		assert_ne!(e1, e2);

		let h1 = &graph.edges[&e1];
		let h2 = &graph.edges[&e2];

		assert_eq!(h1.origin, a);
		assert_eq!(h2.origin, b);

		assert_eq!(h1.twin, e2);
		assert_eq!(h2.twin, e1);

		assert_eq!(h1.next, e2);
		assert_eq!(h2.next, e1);
		assert_eq!(h1.previous, e2);
		assert_eq!(h2.previous, e1);
	}
}
