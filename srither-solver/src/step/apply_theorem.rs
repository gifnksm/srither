use std::collections::HashMap;
use std::rc::Rc;
use std::mem;
use srither_core::geom::{CellId, Geom, Move};
use srither_core::puzzle::{Edge, Puzzle};

use {Error, State, SolverResult};
use model::side_map::SideMap;
use model::theorem::{EdgePattern, Pattern, Theorem, TheoremMatcher, TheoremMatchResult};

#[derive(Clone, Debug)]
struct IndexByEdge {
    points: (CellId, CellId),
    expect_line: Vec<usize>,
    expect_cross: Vec<usize>,
}

#[derive(Debug)]
pub struct TheoremPool {
    counts: Vec<usize>,
    results: Rc<Vec<Vec<EdgePattern<CellId>>>>,
    index_by_edge: Vec<Rc<IndexByEdge>>,
}

impl Clone for TheoremPool {
    fn clone(&self) -> TheoremPool {
        TheoremPool {
            counts: self.counts.clone(),
            results: self.results.clone(),
            index_by_edge: self.index_by_edge.clone(),
        }
    }

    fn clone_from(&mut self, other: &TheoremPool) {
        self.counts.clone_from(&other.counts);
        self.results.clone_from(&other.results);
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

        let counts = matchers.iter().map(|matcher| matcher.num_matcher()).collect();
        let results = matchers.into_iter()
                              .map(|matcher| matcher.result_edges())
                              .collect();
        let edges = map.into_iter()
                       .map(|(points, ex)| {
                           IndexByEdge {
                               points: points,
                               expect_line: ex.0,
                               expect_cross: ex.1,
                           }
                       })
                       .map(Rc::new)
                       .collect();

        Ok(TheoremPool {
            counts: counts,
            results: Rc::new(results),
            index_by_edge: edges,
        })
    }

    fn invalidate(&mut self, i: usize) {
        self.counts[i] = 0;
    }

    fn update(&mut self, i: usize, side_map: &mut SideMap) {
        match self.counts[i] {
            0 => {
                return;
            }
            1 => {
                self.counts[i] = 0;
                for &pat in &self.results[i] {
                    pat.apply(side_map)
                }
            }
            _ => {
                self.counts[i] -= 1;
            }
        }
    }

    pub fn apply_all(&mut self, side_map: &mut SideMap) -> SolverResult<()> {
        unsafe {
            let ptr = self.index_by_edge.as_mut_ptr();

            let mut w = 0;
            for r in 0..self.index_by_edge.len() {
                let read = ptr.offset(r as isize);
                let ibe: &IndexByEdge = &*read;

                match side_map.get_edge(ibe.points.0, ibe.points.1) {
                    State::Fixed(Edge::Cross) => {
                        for &i in &ibe.expect_line {
                            self.invalidate(i);
                        }
                        for &i in &ibe.expect_cross {
                            self.update(i, side_map);
                        }
                    }
                    State::Fixed(Edge::Line) => {
                        for &i in &ibe.expect_line {
                            self.update(i, side_map);
                        }
                        for &i in &ibe.expect_cross {
                            self.invalidate(i);
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
                try!(theo.shift_matches(p - o, puzzle, sum_of_hint, side_map))
                    .update(side_map, &mut data);
            }
        }
    }

    for theo in nonhint_theorem {
        let sz = theo.size();
        for r in (1 - sz.0)..(puzzle.row() + sz.0 - 1) {
            for c in (1 - sz.1)..(puzzle.column() + sz.1 - 1) {
                try!(theo.shift_matches(Move(r, c), puzzle, sum_of_hint, side_map))
                    .update(side_map, &mut data);
            }
        }
    }

    Ok(data)
}

fn apply_all_theorem(matchers: &mut Vec<TheoremMatcher>,
                     side_map: &mut SideMap)
                     -> SolverResult<()> {
    unsafe {
        let ptr = matchers.as_mut_ptr();

        let mut w = 0;
        for r in 0..matchers.len() {
            let read = ptr.offset(r as isize);
            let m = mem::replace(&mut *read, TheoremMatcher::dummy());
            match try!(m.matches(side_map)) {
                TheoremMatchResult::Complete(result) => {
                    for pat in &result {
                        pat.apply(side_map);
                    }
                }
                TheoremMatchResult::Partial(theo) => {
                    let write = ptr.offset(w as isize);
                    *write = theo;
                    w += 1;
                }
                TheoremMatchResult::Conflict => {}
            }
        }

        matchers.truncate(w);
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