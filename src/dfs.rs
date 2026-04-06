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

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::node_index as n;
    use petgraph::visit::{Time, VisitMap, Visitable};
    use std::collections::HashSet;

    /// Parallel edges 0→1, 0→1: first is a tree edge, second is a forward/cross edge.
    #[test]
    fn parallel_edges_tree_and_forward() {
        let gr = petgraph::Graph::<(), ()>::from_edges([(0, 1, ()), (0, 1, ())]);

        let mut tree_edges = vec![];
        let mut cross_forward_edges = vec![];

        depth_first_search(&gr, Some(n(0)), |evt| match evt {
            DfsEvent::TreeEdge(u, v, _) => tree_edges.push((u, v)),
            DfsEvent::CrossForwardEdge(u, v, _) => cross_forward_edges.push((u, v)),
            _ => {}
        });

        assert_eq!(tree_edges, vec![(n(0), n(1))]);
        assert_eq!(cross_forward_edges, vec![(n(0), n(1))]);
    }

    /// Cycle 0→1, 1→0: tree edge to 1, back edge to 0.
    #[test]
    fn cycle_tree_and_back() {
        let gr = petgraph::Graph::<(), ()>::from_edges([(0, 1, ()), (1, 0, ())]);

        let mut tree_edges = vec![];
        let mut back_edges = vec![];

        depth_first_search(&gr, Some(n(0)), |evt| match evt {
            DfsEvent::TreeEdge(u, v, _) => tree_edges.push((u, v)),
            DfsEvent::BackEdge(u, v, _) => back_edges.push((u, v)),
            _ => {}
        });

        assert_eq!(tree_edges, vec![(n(0), n(1))]);
        assert_eq!(back_edges, vec![(n(1), n(0))]);
    }

    fn set<I: IntoIterator>(iter: I) -> HashSet<I::Item>
    where
        I::Item: std::hash::Hash + Eq,
    {
        iter.into_iter().collect()
    }

    /// Parallel of petgraph's tests/graph.rs::dfs_visit, first sub-test:
    /// verify event invariants (discover/finish times, edge classification)
    /// on a hand-crafted weighted directed graph.
    #[test]
    fn event_invariants() {
        let gr = petgraph::Graph::<(), i32>::from_edges([
            (0, 5, 10),
            (0, 2, 20),
            (0, 3, 30),
            (0, 1, 40),
            (1, 3, 50),
            (2, 3, 60),
            (2, 4, 70),
            (4, 0, 80),
            (4, 5, 90),
        ]);

        let invalid_time = Time(!0);
        let mut discover_time = vec![invalid_time; gr.node_count()];
        let mut finish_time = vec![invalid_time; gr.node_count()];
        let mut has_tree_edge = gr.visit_map();
        let mut edges = HashSet::new();

        depth_first_search(&gr, Some(n(0)), |evt| match evt {
            DfsEvent::Discover(n, t) => discover_time[n.index()] = t,
            DfsEvent::Finish(n, t) => finish_time[n.index()] = t,
            DfsEvent::TreeEdge(u, v, w) => {
                assert!(has_tree_edge.visit(v), "Two tree edges to {v:?}!");
                assert!(discover_time[v.index()] == invalid_time);
                assert!(discover_time[u.index()] != invalid_time);
                assert!(finish_time[u.index()] == invalid_time);
                edges.insert((u, v, w));
            }
            DfsEvent::BackEdge(u, v, w) => {
                assert!(discover_time[v.index()] != invalid_time);
                assert!(finish_time[v.index()] == invalid_time);
                edges.insert((u, v, w));
            }
            DfsEvent::CrossForwardEdge(u, v, w) => {
                edges.insert((u, v, w));
            }
        });

        assert!(discover_time.iter().all(|x| *x != invalid_time));
        assert!(finish_time.iter().all(|x| *x != invalid_time));
        assert_eq!(edges.len(), gr.edge_count());
        assert_eq!(
            edges,
            set(gr.edge_references().map(|e| {
                use petgraph::visit::EdgeRef;
                (e.source(), e.target(), *e.weight())
            }))
        );
    }

    /// Parallel of petgraph's tests/graph.rs::dfs_visit, second sub-test:
    /// find a path using Control::Break, verify early termination.
    #[test]
    fn path_finding_with_break() {
        use petgraph::visit::Control;

        let gr = petgraph::Graph::<(), i32>::from_edges([
            (0, 5, 10),
            (0, 2, 20),
            (0, 3, 30),
            (0, 1, 40),
            (1, 3, 50),
            (2, 3, 60),
            (2, 4, 70),
            (4, 0, 80),
            (4, 5, 90),
        ]);

        let mut predecessor = vec![petgraph::graph::NodeIndex::end(); gr.node_count()];
        let start = n(0);
        let goal = n(4);
        let ret = depth_first_search(&gr, Some(start), |event| {
            if let DfsEvent::TreeEdge(u, v, _) = event {
                predecessor[v.index()] = u;
                if v == goal {
                    return Control::Break(u);
                }
            }
            Control::Continue
        });
        assert!(ret.break_value().is_some());
        assert!(
            predecessor
                .iter()
                .any(|x| *x == petgraph::graph::NodeIndex::end())
        );

        let mut next = goal;
        let mut path = vec![next];
        while next != start {
            let pred = predecessor[next.index()];
            path.push(pred);
            next = pred;
        }
        path.reverse();
        assert_eq!(&path, &[n(0), n(2), n(4)]);
    }

    /// Parallel of petgraph's tests/graph.rs::dfs_visit, third sub-test:
    /// prune a node and verify its subtree is not reached.
    #[test]
    fn pruning() {
        use petgraph::visit::Control;

        let gr = petgraph::Graph::<(), i32>::from_edges([
            (0, 5, 10),
            (0, 2, 20),
            (0, 3, 30),
            (0, 1, 40),
            (1, 3, 50),
            (2, 3, 60),
            (2, 4, 70),
            (4, 0, 80),
            (4, 5, 90),
        ]);

        let start = n(0);
        let prune = n(2);
        let nongoal = n(4);
        let ret = depth_first_search(&gr, Some(start), |event| {
            if let DfsEvent::Discover(n, _) = event {
                if n == prune {
                    return Control::Prune;
                }
            } else if let DfsEvent::TreeEdge(u, v, _) = event {
                if v == nongoal {
                    return Control::Break(u);
                }
            }
            Control::Continue
        });
        assert!(ret.break_value().is_none());
    }

    /// Parallel of petgraph's tests/quickcheck.rs::dfs_visit:
    /// property-based test verifying event invariants on random weighted graphs.
    #[quickcheck_macros::quickcheck]
    fn event_invariants_quickcheck(gr: petgraph::Graph<(), i32>, node: usize) -> bool {
        if gr.node_count() == 0 {
            return true;
        }
        let start_node = petgraph::graph::node_index(node % gr.node_count());

        let invalid_time = Time(!0);
        let mut discover_time = vec![invalid_time; gr.node_count()];
        let mut finish_time = vec![invalid_time; gr.node_count()];
        let mut has_tree_edge = gr.visit_map();
        let mut edges = HashSet::new();

        depth_first_search(
            &gr,
            Some(start_node).into_iter().chain(gr.node_indices()),
            |evt| match evt {
                DfsEvent::Discover(n, t) => discover_time[n.index()] = t,
                DfsEvent::Finish(n, t) => finish_time[n.index()] = t,
                DfsEvent::TreeEdge(u, v, w) => {
                    assert!(has_tree_edge.visit(v), "Two tree edges to {v:?}!");
                    assert!(discover_time[v.index()] == invalid_time);
                    assert!(discover_time[u.index()] != invalid_time);
                    assert!(finish_time[u.index()] == invalid_time);
                    edges.insert((u, v, w));
                }
                DfsEvent::BackEdge(u, v, w) => {
                    assert!(discover_time[v.index()] != invalid_time);
                    assert!(finish_time[v.index()] == invalid_time);
                    edges.insert((u, v, w));
                }
                DfsEvent::CrossForwardEdge(u, v, w) => {
                    edges.insert((u, v, w));
                }
            },
        );

        assert!(discover_time.iter().all(|x| *x != invalid_time));
        assert!(finish_time.iter().all(|x| *x != invalid_time));
        assert_eq!(edges.len(), gr.edge_count());
        assert_eq!(
            edges,
            set(gr.edge_references().map(|e| {
                use petgraph::visit::EdgeRef;
                (e.source(), e.target(), *e.weight())
            }))
        );
        true
    }

    /// Undirected DFS produces only tree edges and back edges — never cross/forward edges.
    #[quickcheck_macros::quickcheck]
    fn undirected_no_cross_forward(
        gr: petgraph::Graph<(), i32, petgraph::Undirected>,
        node: usize,
    ) -> bool {
        if gr.node_count() == 0 {
            return true;
        }
        let start_node = petgraph::graph::node_index(node % gr.node_count());

        let mut saw_cross_forward = false;
        depth_first_search(
            &gr,
            Some(start_node).into_iter().chain(gr.node_indices()),
            |evt| {
                if let DfsEvent::CrossForwardEdge(..) = evt {
                    saw_cross_forward = true;
                }
            },
        );
        !saw_cross_forward
    }

    /// Parenthesis theorem: for any two nodes u, v the discover/finish intervals
    /// [d(u), f(u)] and [d(v), f(v)] are either disjoint or one contains the other.
    #[quickcheck_macros::quickcheck]
    fn parenthesis_theorem(gr: petgraph::Graph<(), i32>, node: usize) -> bool {
        if gr.node_count() == 0 {
            return true;
        }
        let start_node = petgraph::graph::node_index(node % gr.node_count());

        let n = gr.node_count();
        let invalid_time = Time(!0);
        let mut discover_time = vec![invalid_time; n];
        let mut finish_time = vec![invalid_time; n];
        let mut parent = vec![usize::MAX; n];

        depth_first_search(
            &gr,
            Some(start_node).into_iter().chain(gr.node_indices()),
            |evt| match evt {
                DfsEvent::Discover(n, t) => discover_time[n.index()] = t,
                DfsEvent::Finish(n, t) => finish_time[n.index()] = t,
                DfsEvent::TreeEdge(u, v, _) => parent[v.index()] = u.index(),
                _ => {}
            },
        );

        let is_ancestor = |ancestor: usize, mut descendant: usize| -> bool {
            while descendant != usize::MAX {
                if descendant == ancestor {
                    return true;
                }
                descendant = parent[descendant];
            }
            false
        };

        for u in 0..n {
            let (du, fu) = (discover_time[u].0, finish_time[u].0);
            for v in (u + 1)..n {
                let (dv, fv) = (discover_time[v].0, finish_time[v].0);
                let disjoint = fu < dv || fv < du;
                let u_contains_v = du < dv && fv < fu;
                let v_contains_u = dv < du && fu < fv;
                // Intervals are either disjoint or one contains the other.
                assert!(
                    disjoint || u_contains_v || v_contains_u,
                    "Partial overlap for nodes {u} and {v}"
                );
                // Containment iff ancestor relationship.
                assert_eq!(
                    u_contains_v,
                    is_ancestor(u, v),
                    "u={u} contains v={v} but not ancestor"
                );
                assert_eq!(
                    v_contains_u,
                    is_ancestor(v, u),
                    "v={v} contains u={u} but not ancestor"
                );
            }
        }
        true
    }
}
