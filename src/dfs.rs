/// A DFS visitor event that includes edge weights.
///
/// For edge events, `parent` is the node closer to the tree root and `child`
/// is the node being reached, regardless of how the edge is stored in the graph.
#[derive(Copy, Clone, Debug)]
pub enum DfsEvent<N, W> {
    Discover(N, petgraph::visit::Time),
    TreeEdge(N, N, W),
    BackEdge(N, N, W),
    CrossForwardEdge(N, N, W),
    Finish(N, petgraph::visit::Time),
}

/// A depth first search that provides edge weights in visitor events.
///
/// Mirrors [`petgraph::visit::depth_first_search`] but requires `IntoEdges`
/// instead of `IntoNeighbors`, so that edge weights are available.
pub fn depth_first_search<G, I, F, C>(_graph: G, _starts: I, _visitor: F) -> C
where
    G: petgraph::visit::IntoEdges + petgraph::visit::Visitable,
    G::EdgeWeight: Copy,
    I: IntoIterator<Item = G::NodeId>,
    F: FnMut(DfsEvent<G::NodeId, G::EdgeWeight>) -> C,
    C: petgraph::visit::ControlFlow,
{
    todo!()
}
