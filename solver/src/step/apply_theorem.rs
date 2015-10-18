use std::collections::HashMap;
use std::rc::Rc;
use std::mem;
use slsr_core::geom::{CellId, Geom, Move};
use slsr_core::puzzle::{Edge, Puzzle};

use {Error, State, SolverResult};
use model::side_map::SideMap;
use model::theorem::{Pattern, Theorem, TheoremMatcher};

#[derive(Clone, Debug)]
struct TheoremCount {
    rest_count: usize,
    result: Option<Rc<Vec<(Edge, (CellId, CellId))>>>,
}

impl From<TheoremMatcher> for TheoremCount {
    fn from(matcher: TheoremMatcher) -> TheoremCount {
        TheoremCount {
            rest_count: matcher.num_matcher(),
            result: Some(Rc::new(matcher.result_edges().collect())),
        }
    }
}

impl TheoremCount {
    fn invalidate(&mut self) {
        self.rest_count = 0;
        self.result = None;
    }

    fn update(&mut self, side_map: &mut SideMap) {
        match self.rest_count {
            0 => {
                return;
            }
            1 => {
                self.rest_count = 0;
                for &(edge, points) in &*self.result.take().unwrap() {
                    let _ = side_map.set_edge(points.0, points.1, edge);
                }
            }
            _ => {
                self.rest_count -= 1;
            }
        }
    }
}

#[derive(Clone, Debug)]
struct IndexByEdge {
    points: (CellId, CellId),
    expect_line: Rc<Vec<usize>>,
    expect_cross: Rc<Vec<usize>>,
}

impl IndexByEdge {
    fn new(points: (CellId, CellId),
           expect_line: Vec<usize>,
           expect_cross: Vec<usize>)
           -> IndexByEdge {
        IndexByEdge {
            points: points,
            expect_line: Rc::new(expect_line),
            expect_cross: Rc::new(expect_cross),
        }
    }
}

#[derive(Debug)]
pub struct TheoremPool {
    matchers: Vec<TheoremCount>,
    index_by_edge: Vec<IndexByEdge>,
}

impl Clone for TheoremPool {
    fn clone(&self) -> TheoremPool {
        TheoremPool {
            matchers: self.matchers.clone(),
            index_by_edge: self.index_by_edge.clone(),
        }
    }

    fn clone_from(&mut self, other: &TheoremPool) {
        self.matchers.clone_from(&other.matchers);
        self.index_by_edge.clone_from(&other.index_by_edge);
    }
}

impl TheoremPool {
    pub fn new<'a, T>(theo_defs: T,
                      puzzle: &Puzzle,
                      sum_of_hint: u32,
                      side_map: &mut SideMap)
                      -> SolverResult<TheoremPool>
        where T: IntoIterator<Item = Theorem>
    {
        let mut matchers = try!(create_matcher_list(theo_defs, puzzle, sum_of_hint, side_map));

        loop {
            let rev = side_map.revision();

            try!(apply_all_theorem(&mut matchers, side_map));
            if side_map.revision() != rev {
                continue;
            }

            break;
        }

        merge_duplicate_matchers(&mut matchers);

        let mut map = HashMap::new();
        for (i, m) in matchers.iter().enumerate() {
            for (edge, points) in m.matcher_edges() {
                let e = map.entry(points).or_insert((vec![], vec![]));
                match edge {
                    Edge::Line => e.0.push(i),
                    Edge::Cross => e.1.push(i),
                }
            }
        }

        let matchers = matchers.into_iter().map(From::from).collect();
        let edges = map.into_iter()
                       .map(|(points, ex)| IndexByEdge::new(points, ex.0, ex.1))
                       .collect();

        Ok(TheoremPool {
            matchers: matchers,
            index_by_edge: edges,
        })
    }

    pub fn apply_all(&mut self, side_map: &mut SideMap) -> SolverResult<()> {
        unsafe {
            let ptr = self.index_by_edge.as_mut_ptr();

            let mut w = 0;
            for r in 0..self.index_by_edge.len() {
                let read = ptr.offset(r as isize);
                let ibe = &*read;

                match side_map.get_edge(ibe.points.0, ibe.points.1) {
                    State::Fixed(Edge::Cross) => {
                        for &i in &*ibe.expect_line {
                            self.matchers[i].invalidate();
                        }
                        for &i in &*ibe.expect_cross {
                            self.matchers[i].update(side_map);
                        }
                    }
                    State::Fixed(Edge::Line) => {
                        for &i in &*ibe.expect_line {
                            self.matchers[i].update(side_map);
                        }
                        for &i in &*ibe.expect_cross {
                            self.matchers[i].invalidate();
                        }
                    }
                    State::Unknown => {
                        let write = ptr.offset(w as isize);
                        mem::swap(&mut *write, &mut *read);
                        w += 1;
                    }
                    State::Conflict => {
                        return Err(Error::invalid_board());
                    }
                }
            }

            self.index_by_edge.truncate(w);
        }

        Ok(())
    }
}

fn create_matcher_list<'a, T>(theo_defs: T,
                              puzzle: &Puzzle,
                              sum_of_hint: u32,
                              side_map: &mut SideMap)
                              -> SolverResult<Vec<TheoremMatcher>>
    where T: IntoIterator<Item = Theorem>
{
    let it = theo_defs.into_iter().flat_map(|theo| theo.all_rotations());

    let mut hint_theorem = [vec![], vec![], vec![], vec![], vec![]];
    let mut nonhint_theorem = vec![];

    for theo in it {
        match theo.head() {
            Pattern::Hint(h) => hint_theorem[h.hint() as usize].push(theo),
            _ => nonhint_theorem.push(theo),
        }
    }

    let mut data = vec![];

    for p in puzzle.points() {
        if let Some(x) = puzzle.hint(p) {
            for theo in &hint_theorem[x as usize] {
                let o = match theo.head() {
                    Pattern::Hint(hint) => hint.point(),
                    _ => panic!(),
                };
                let matcher = theo.clone().shift(p - o);
                try!(matcher.matches(puzzle, sum_of_hint, side_map)).update(side_map, &mut data);
            }
        }
    }

    for theo in nonhint_theorem {
        let sz = theo.size();
        for r in (1 - sz.0)..(puzzle.row() + sz.0 - 1) {
            for c in (1 - sz.1)..(puzzle.column() + sz.1 - 1) {
                let matcher = theo.clone().shift(Move(r, c));
                try!(matcher.matches(puzzle, sum_of_hint, side_map)).update(side_map, &mut data);
            }
        }
    }

    Ok(data)
}

fn apply_all_theorem(matchers: &mut Vec<TheoremMatcher>,
                     side_map: &mut SideMap)
                     -> SolverResult<()> {
    let cap = matchers.len();

    for m in mem::replace(matchers, Vec::with_capacity(cap)) {
        try!(m.matches(side_map)).update(side_map, matchers);
    }

    Ok(())
}

fn merge_duplicate_matchers(matchers: &mut Vec<TheoremMatcher>) {
    matchers.sort();

    // Merge elements that have same matchers.
    unsafe {
        let ptr = matchers.as_mut_ptr();

        let mut w = 1;
        for r in 1..matchers.len() {
            let read = ptr.offset(r as isize);
            let cmp = ptr.offset((w - 1) as isize);

            match (*cmp).merge(&*read) {
                Ok(()) => {}
                Err(()) => {
                    let write = cmp.offset(1);
                    mem::swap(&mut *write, &mut *read);
                    w += 1;
                }
            }
        }

        matchers.truncate(w);
    }
}
