use std::cmp;
use slsr_core::board::Side;
use slsr_core::geom::{CellId, Geom, OUTSIDE_CELL_ID};

use ::{State, SolverResult};
use ::model::connect_map::ConnectMap;
use ::model::side_map::SideMap;

fn create_conn_graph(conn_map: &mut ConnectMap, filter_side: Side)
                     -> (Vec<CellId>, Vec<Vec<usize>>)
{
    let mut pts = vec![];
    if filter_side != Side::Out {
        pts.push(OUTSIDE_CELL_ID)
    }

    for i in 0..conn_map.cell_len() {
        let p = CellId::new(i);
        let a = conn_map.get(p);
        if a.coord() == p && a.side() != State::Fixed(filter_side) {
            pts.push(p);
        }
    }

    pts.sort();

    let graph = pts.iter().map(|&p| {
        conn_map.get(p).unknown_edge().iter()
            .filter_map(|&p2| pts.binary_search(&p2).ok())
            .collect::<Vec<_>>()
    }).collect();

    (pts, graph)
}

fn get_articulation(graph: &[Vec<usize>], v: usize) -> (Vec<usize>, Vec<bool>) {
    if graph.is_empty() { return (vec![], vec![]) }

    let mut visited = vec![false; graph.len()];
    let mut ord = vec![0; graph.len()];
    let mut low = vec![0; graph.len()];
    let mut arts = vec![];
    let mut ord_cnt = 0;
    dfs(graph, v, &mut visited, &mut ord, &mut low, &mut ord_cnt, &mut arts);

    fn dfs(graph: &[Vec<usize>],
           v: usize, visited: &mut [bool], ord: &mut [usize], low: &mut [usize], 
           ord_cnt: &mut usize, arts: &mut Vec<usize>) {
        debug_assert!(!visited[v]);

        *ord_cnt += 1;
        visited[v] = true;
        ord[v] = *ord_cnt;
        low[v] = ord[v];

        let mut is_articulation = false;
        let mut num_child = 0;

        for &u in &graph[v] {
            if u == v { continue }

            if !visited[u] {
                dfs(graph, u, visited, ord, low, ord_cnt, arts);

                num_child += 1;
                low[v] = cmp::min(low[v], low[u]);
                if ord[v] != 1 && ord[v] <= low[u] {
                    is_articulation = true;
                }
            } else {
                low[v] = cmp::min(low[v], ord[u]);
            }
        }

        if ord[v] == 1 && num_child > 1 {
            is_articulation = true;
        }

        if is_articulation {
            arts.push(v);
        }
    }

    (arts, visited)
}

fn find_disconn_area(conn_map: &mut ConnectMap, pts: &[CellId], visited: &[bool])
                     -> SolverResult<Vec<usize>>
{
    let mut disconn = vec![];
    for (u, &vis) in visited.iter().enumerate() {
        if !vis { disconn.push(u); }
    }
    if disconn.is_empty() {
        // All area is connected.
        return Ok(disconn)
    }

    let mut sum = 0;
    for &v in &disconn {
        sum += conn_map.get(pts[v]).sum_of_hint();
    }
    if sum == 0 {
        // Disconnected components does not contain any edges. It is a hole in
        // the filter_side area.
        return Ok(disconn)
    }

    let mut conn = vec![];
    for (u, &vis) in visited.iter().enumerate() {
        if vis { conn.push(u); }
    }
    let mut sum = 0;
    for &v in &conn {
        sum += conn_map.get(pts[v]).sum_of_hint();
    }
    if sum == 0 {
        // Conencted area does not contain any edges. It is a hole in the
        // filter_side area.
        return Ok(conn)
    }

    // Graph is splitted into more than two parts, but both parts contain edges.
    // This may be valid in some situation, so, return empty.
    Ok(vec![])
}

fn splits(graph: &[Vec<usize>], v: usize,
          conn_map: &mut ConnectMap, pts: &[CellId], side: Side) -> bool {
    if graph.is_empty() { return false }

    let mut contain_cnt = 0;
    let mut visited = vec![false; graph.len()];

    visited[v] = true;

    for &u in &graph[v] {
        if u == v || visited[u] { continue }

        if dfs(graph, u, &mut visited, conn_map, pts, side) {
            contain_cnt += 1;
        }
    }

    fn dfs(graph: &[Vec<usize>], v: usize, visited: &mut [bool],
           conn_map: &mut ConnectMap, pts: &[CellId], side: Side) -> bool {
        let mut contains = conn_map.get(pts[v]).side() == State::Fixed(side);
        visited[v] = true;

        for &u in &graph[v] {
            if u == v || visited[u] { continue }
            contains |= dfs(graph, u, visited, conn_map, pts, side);
        }
        contains
    }

    contain_cnt > 1
}

pub fn run(side_map: &mut SideMap, conn_map: &mut ConnectMap)
    -> SolverResult<()>
{
    try!(conn_map.sync(side_map));

    let sides = &[(Side::In, Side::Out),
                  (Side::Out, Side::In)];

    for &(set_side, filter_side) in sides {
        let (pts, graph) = create_conn_graph(conn_map, filter_side);
        let (arts, visited) = get_articulation(&graph, 0);

        let disconn = try!(find_disconn_area(conn_map, &pts, &visited));
        for &v in &disconn {
            side_map.set_side(pts[v], filter_side);
        }
        for &v in &arts {
            let p = pts[v];

            if conn_map.get(p).side() != State::Fixed(set_side) &&
                splits(&graph, v, conn_map, &pts, set_side)
            {
                side_map.set_side(p, set_side);
            }
        }
    }

    Ok(())
}
