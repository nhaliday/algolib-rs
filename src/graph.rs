use proptest::prelude::Strategy;

/// A DFS visitor event that includes edge weights.
///
/// For edge events, `parent` is the node closer to the tree root and `child`
/// is the node being reached, regardless of how the edge is stored in the graph.
#[derive(Copy, Clone, Debug)]
pub enum DfsEvent<N, W> {
    Discover(N, petgraph::visit::Time),
    TreeEdge(N, N, W),
    BackEdge(N, N, W),
    ForwardEdge(N, N, W),
    CrossEdge(N, N, W),
    Finish(N, petgraph::visit::Time),
}

/// A depth first search that provides edge weights in visitor events.
///
/// Mirrors [`petgraph::visit::depth_first_search`] but requires `IntoEdges`
/// instead of `IntoNeighbors`, so that edge weights are available.
pub fn depth_first_search<G, I, F, C>(graph: G, starts: I, mut visitor: F) -> C
where
    G: petgraph::visit::IntoEdges + petgraph::visit::Visitable + petgraph::visit::NodeIndexable,
    G::EdgeWeight: Copy,
    I: IntoIterator<Item = G::NodeId>,
    F: FnMut(DfsEvent<G::NodeId, G::EdgeWeight>) -> C,
    C: petgraph::visit::ControlFlow,
{
    use petgraph::visit::{EdgeRef, Time, VisitMap};

    let mut time = Time(0);
    let mut discovered = graph.visit_map();
    let mut finished = graph.visit_map();
    let mut discover_time = vec![Time(usize::MAX); graph.node_bound()];

    let mut stack: Vec<(G::NodeId, <G as petgraph::visit::IntoEdges>::Edges, bool)> = Vec::new();

    for start in starts {
        if !discovered.visit(start) {
            continue;
        }
        let t = time_post_inc(&mut time);
        discover_time[graph.to_index(start)] = t;
        let c = visitor(DfsEvent::Discover(start, t));
        if c.should_break() {
            return c;
        }
        let pruned = c.should_prune();
        stack.push((start, graph.edges(start), pruned));

        while let Some(&mut (u, ref mut edges, pruned)) = stack.last_mut() {
            let mut next = || {
                edges.next().map(|e| {
                    let v = if e.source() == u {
                        e.target()
                    } else {
                        e.source()
                    };
                    (v, *e.weight())
                })
            };
            if !pruned && let Some((v, w)) = next() {
                if !discovered.is_visited(&v) {
                    let c = visitor(DfsEvent::TreeEdge(u, v, w));
                    if c.should_break() {
                        return c;
                    }
                    if c.should_prune() {
                        continue;
                    }
                    discovered.visit(v);
                    let t = time_post_inc(&mut time);
                    discover_time[graph.to_index(v)] = t;
                    let c = visitor(DfsEvent::Discover(v, t));
                    if c.should_break() {
                        return c;
                    }
                    let pruned = c.should_prune();
                    stack.push((v, graph.edges(v), pruned));
                } else if !finished.is_visited(&v) {
                    let c = visitor(DfsEvent::BackEdge(u, v, w));
                    if c.should_break() {
                        return c;
                    }
                } else if discover_time[graph.to_index(u)] < discover_time[graph.to_index(v)] {
                    let c = visitor(DfsEvent::ForwardEdge(u, v, w));
                    if c.should_break() {
                        return c;
                    }
                } else {
                    let c = visitor(DfsEvent::CrossEdge(u, v, w));
                    if c.should_break() {
                        return c;
                    }
                }
            } else {
                let (u, _, _) = stack.pop().unwrap();
                finished.visit(u);
                let c = visitor(DfsEvent::Finish(u, time));
                if c.should_break() {
                    return c;
                }
            }
        }
    }
    C::continuing()
}

fn time_post_inc(time: &mut petgraph::visit::Time) -> petgraph::visit::Time {
    let t = *time;
    time.0 += 1;
    t
}

#[derive(Debug, thiserror::Error)]
pub enum PruferEncodeError {
    #[error("Prüfer encoding requires at least 2 nodes, got {node_count}")]
    TooFewNodes { node_count: usize },
    #[error("expected {expected} edges for a tree on {node_count} nodes, got {actual}")]
    WrongEdgeCount {
        node_count: usize,
        expected: usize,
        actual: usize,
    },
    #[error("graph is not connected")]
    Disconnected,
}

/// Encode a labeled tree as its Prüfer sequence.
///
/// The input must be a tree: an undirected, connected graph on nodes `0..n`
/// with exactly `n - 1` edges. Returns an error if any of these conditions
/// are violated.
///
/// Runs in O(n) time using the "pointer" algorithm of Heinz Prüfer (1918).
pub fn prufer_encode(
    tree: &petgraph::graph::UnGraph<(), (), usize>,
) -> Result<Vec<usize>, PruferEncodeError> {
    let n = tree.node_count();
    if n < 2 {
        return Err(PruferEncodeError::TooFewNodes { node_count: n });
    }
    let expected_edge_count = n - 1;
    if tree.edge_count() != expected_edge_count {
        return Err(PruferEncodeError::WrongEdgeCount {
            node_count: n,
            expected: expected_edge_count,
            actual: tree.edge_count(),
        });
    }
    // Check connectivity via DFS.
    let mut count = 0usize;
    petgraph::visit::depth_first_search(tree, Some(petgraph::graph::NodeIndex::new(0)), |event| {
        if let petgraph::visit::DfsEvent::Discover(_, _) = event {
            count += 1;
        }
    });
    if count != n {
        return Err(PruferEncodeError::Disconnected);
    }
    if n == 2 {
        return Ok(vec![]);
    }
    let mut degree = vec![0usize; n];
    for edge in tree.edge_references() {
        use petgraph::visit::EdgeRef;
        degree[edge.source().index()] += 1;
        degree[edge.target().index()] += 1;
    }

    let mut sequence = Vec::with_capacity(n - 2);
    // Pointer to the smallest leaf candidate.
    let mut leaf = degree.iter().position(|&d| d == 1).unwrap();

    for _ in 0..n - 2 {
        // Find the neighbor of the current leaf.
        let neighbor = tree
            .neighbors(petgraph::graph::NodeIndex::new(leaf))
            .find(|&v| degree[v.index()] > 0)
            .unwrap()
            .index();
        sequence.push(neighbor);
        degree[leaf] = 0;
        degree[neighbor] -= 1;

        // If the neighbor became a leaf and is smaller than the current pointer,
        // use it directly (avoids scanning forward).
        if degree[neighbor] == 1 && neighbor < leaf {
            leaf = neighbor;
        } else {
            // Advance the pointer to the next leaf.
            leaf += 1;
            while leaf < n && degree[leaf] != 1 {
                leaf += 1;
            }
        }
    }
    Ok(sequence)
}

/// Decode a Prüfer sequence into a labeled tree.
///
/// `sequence` must contain values in `0..n` where `n = sequence.len() + 2`.
/// Returns an undirected graph on `n` nodes with `n - 1` edges.
///
/// Runs in O(n) time using the inverse of the encoding pointer algorithm.
pub fn prufer_decode(sequence: &[usize]) -> petgraph::graph::UnGraph<(), (), usize> {
    let n = sequence.len() + 2;
    let mut graph = petgraph::Graph::with_capacity(n, n.saturating_sub(1));
    graph.extend_with_edges(std::iter::empty::<(usize, usize)>());
    for _ in 0..n {
        graph.add_node(());
    }

    if n == 2 {
        graph.add_edge(
            petgraph::graph::NodeIndex::new(0),
            petgraph::graph::NodeIndex::new(1),
            (),
        );
        return graph;
    }

    let mut degree = vec![1usize; n];
    for &v in sequence {
        degree[v] += 1;
    }

    let mut leaf = degree.iter().position(|&d| d == 1).unwrap();

    for &v in sequence {
        graph.add_edge(
            petgraph::graph::NodeIndex::new(leaf),
            petgraph::graph::NodeIndex::new(v),
            (),
        );
        degree[leaf] = 0;
        degree[v] -= 1;

        if degree[v] == 1 && v < leaf {
            leaf = v;
        } else {
            leaf += 1;
            while leaf < n && degree[leaf] != 1 {
                leaf += 1;
            }
        }
    }

    // Connect the last two remaining nodes.
    let remaining: Vec<usize> = degree
        .iter()
        .enumerate()
        .filter(|(_, d)| **d == 1)
        .map(|(i, _)| i)
        .collect();
    debug_assert_eq!(remaining.len(), 2);
    graph.add_edge(
        petgraph::graph::NodeIndex::new(remaining[0]),
        petgraph::graph::NodeIndex::new(remaining[1]),
        (),
    );

    graph
}

pub fn arbitrary_tree(
    n: usize,
) -> impl proptest::strategy::Strategy<Value = petgraph::graph::UnGraph<(), (), usize>> {
    if n <= 1 {
        let mut g = petgraph::graph::UnGraph::<(), (), usize>::default();
        for _ in 0..n {
            g.add_node(());
        }
        proptest::strategy::Strategy::boxed(proptest::strategy::Just(g))
    } else {
        proptest::strategy::Strategy::boxed(
            proptest::collection::vec(0..n, n - 2).prop_map(|seq| prufer_decode(&seq)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::node_index as n;
    use petgraph::visit::{Time, VisitMap, Visitable};
    use std::collections::HashSet;

    /// Parallel edges 0→1, 0→1: first is a tree edge, second is a forward/cross edge.
    #[test]
    fn dfs_classifies_parallel_edges_as_one_tree_and_one_forward() {
        let gr = petgraph::Graph::<(), ()>::from_edges([(0, 1, ()), (0, 1, ())]);

        let mut tree_edges = vec![];
        let mut forward_edges = vec![];

        depth_first_search(&gr, Some(n(0)), |evt| match evt {
            DfsEvent::TreeEdge(u, v, _) => tree_edges.push((u, v)),
            DfsEvent::ForwardEdge(u, v, _) => forward_edges.push((u, v)),
            DfsEvent::CrossEdge(_, _, _) | DfsEvent::BackEdge(_, _, _) => unreachable!(),
            _ => {}
        });

        assert_eq!(tree_edges, vec![(n(0), n(1))]);
        assert_eq!(forward_edges, vec![(n(0), n(1))]);
    }

    /// Cycle 0→1, 1→0: tree edge to 1, back edge to 0.
    #[test]
    fn dfs_classifies_cycle_as_one_tree_and_one_back() {
        let gr = petgraph::Graph::<(), ()>::from_edges([(0, 1, ()), (1, 0, ())]);

        let mut tree_edges = vec![];
        let mut back_edges = vec![];

        depth_first_search(&gr, Some(n(0)), |evt| match evt {
            DfsEvent::TreeEdge(u, v, _) => tree_edges.push((u, v)),
            DfsEvent::BackEdge(u, v, _) => back_edges.push((u, v)),
            DfsEvent::CrossEdge(_, _, _) | DfsEvent::ForwardEdge(_, _, _) => unreachable!(),
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
    fn dfs_events_satisfy_invariants() {
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
            DfsEvent::ForwardEdge(u, v, w) | DfsEvent::CrossEdge(u, v, w) => {
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
    fn dfs_break_finds_path_via_early_termination() {
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
    fn dfs_prune_skips_subtree() {
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
    fn dfs_random_graphs_satisfy_event_invariants(
        gr: petgraph::Graph<(), i32>,
        node: usize,
    ) -> bool {
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
                DfsEvent::ForwardEdge(u, v, w) | DfsEvent::CrossEdge(u, v, w) => {
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

    /// Parenthesis theorem: for any two nodes u, v the discover/finish intervals
    /// [d(u), f(u)] and [d(v), f(v)] are either disjoint or one contains the other.
    #[quickcheck_macros::quickcheck]
    fn dfs_discover_finish_intervals_satisfy_parenthesis_theorem(
        gr: petgraph::Graph<(), ()>,
        node: usize,
    ) -> bool {
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
                let disjoint = fu <= dv || fv <= du;
                let u_contains_v = du < dv && fv <= fu;
                let v_contains_u = dv < du && fu <= fv;
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

    /// In undirected DFS, each non-self-loop edge is seen from both endpoints:
    /// once as a BackEdge (descendant → ancestor) and once as a TreeEdge or ForwardEdge
    /// (ancestor → descendant). These should be in exact bijection.
    #[quickcheck_macros::quickcheck]
    fn dfs_undirected_back_edges_biject_with_tree_and_forward(
        gr: petgraph::Graph<(), (), petgraph::Undirected>,
    ) -> bool {
        type NormalizedEdge = (usize, usize);
        let normalize = |u: petgraph::graph::NodeIndex, v: petgraph::graph::NodeIndex| {
            (
                std::cmp::min(u.index(), v.index()),
                std::cmp::max(u.index(), v.index()),
            )
        };

        let mut tree_and_forward_edges: Vec<NormalizedEdge> = Vec::new();
        let mut back_edges: Vec<NormalizedEdge> = Vec::new();

        depth_first_search(&gr, gr.node_indices(), |event| match event {
            DfsEvent::TreeEdge(u, v, _) | DfsEvent::ForwardEdge(u, v, _) => {
                tree_and_forward_edges.push(normalize(u, v));
            }
            DfsEvent::BackEdge(u, v, _) if u != v => back_edges.push(normalize(u, v)),
            DfsEvent::CrossEdge(_, _, _) => unreachable!(),
            _ => {}
        });

        tree_and_forward_edges.sort();
        back_edges.sort();

        back_edges == tree_and_forward_edges
    }

    /// Prüfer encoding of the path 0-1-2-3-4 is [1, 2, 3].
    #[test]
    fn prufer_encode_produces_correct_path_sequence() {
        let graph =
            petgraph::graph::UnGraph::<(), (), usize>::from_edges([(0, 1), (1, 2), (2, 3), (3, 4)]);
        assert_eq!(prufer_encode(&graph).unwrap(), vec![1, 2, 3]);
    }

    /// Prüfer encoding of the star graph (center=0, leaves=1..5) is [0, 0, 0].
    #[test]
    fn prufer_encode_produces_correct_star_sequence() {
        let graph =
            petgraph::graph::UnGraph::<(), (), usize>::from_edges([(0, 1), (0, 2), (0, 3), (0, 4)]);
        assert_eq!(prufer_encode(&graph).unwrap(), vec![0, 0, 0]);
    }

    /// Decode([1, 2, 3]) reconstructs a path graph, and re-encoding yields [1, 2, 3].
    #[test]
    fn prufer_decode_roundtrips_path_sequence() {
        let sequence = vec![1, 2, 3];
        let graph = prufer_decode(&sequence);
        assert_eq!(graph.node_count(), 5);
        assert_eq!(graph.edge_count(), 4);
        assert_eq!(prufer_encode(&graph).unwrap(), sequence);
    }

    /// Edge case: empty sequence decodes to a single-edge tree on 2 nodes.
    #[test]
    fn prufer_decode_handles_two_node_edge_case() {
        let empty: Vec<usize> = vec![];

        let g = prufer_decode(&empty[..]);
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(prufer_encode(&g).unwrap(), empty);
    }

    #[test]
    fn prufer_encode_decode_roundtrips_arbitrary_sequences() {
        let n = 20;
        let strategy = proptest::collection::vec(0..n, n - 2);

        let mut test_runner =
            proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
                source_file: Some(file!()),
                ..proptest::test_runner::Config::default()
            });
        test_runner
            .run(&strategy, |sequence| {
                let graph = prufer_decode(&sequence);
                proptest::prop_assert_eq!(graph.node_count(), n);
                proptest::prop_assert_eq!(graph.edge_count(), n - 1);
                let re_encoded = prufer_encode(&graph).unwrap();
                proptest::prop_assert_eq!(re_encoded, sequence);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn prufer_encode_handles_arbitrary_tree() {
        let mut test_runner =
            proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
                source_file: Some(file!()),
                ..proptest::test_runner::Config::default()
            });

        test_runner
            .run(&arbitrary_tree(20), |tree| {
                proptest::prop_assert!(prufer_encode(&tree).is_ok());
                Ok(())
            })
            .unwrap();
    }
}
