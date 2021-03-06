// Copyright (c) 2016 srither-solver developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

use srither_core::puzzle::{Puzzle, Side};
use srither_core::geom::{CellId, Geom};

use {Error, SolverResult};
use model::{ConnectMap, SideMap, State, Theorem, TheoremPool};

#[derive(Debug)]
pub struct Solver<'a> {
    puzzle: &'a Puzzle,
    sum_of_hint: u32,
    theorem_pool: TheoremPool,
    side_map: SideMap,
    connect_map: Option<ConnectMap>,
    conn_analysis_revision: Option<u32>,
}

impl<'a> Clone for Solver<'a> {
    fn clone(&self) -> Solver<'a> {
        Solver {
            puzzle: self.puzzle,
            sum_of_hint: self.sum_of_hint,
            theorem_pool: self.theorem_pool.clone(),
            side_map: self.side_map.clone(),
            connect_map: self.connect_map.clone(),
            conn_analysis_revision: self.conn_analysis_revision,
        }
    }

    fn clone_from(&mut self, other: &Solver<'a>) {
        self.puzzle = other.puzzle;
        self.sum_of_hint = other.sum_of_hint;
        self.theorem_pool.clone_from(&other.theorem_pool);
        self.side_map.clone_from(&other.side_map);
        self.connect_map.clone_from(&other.connect_map);
        self.conn_analysis_revision = other.conn_analysis_revision;
    }
}

impl<'a> Solver<'a> {
    pub fn new<I>(puzzle: &'a Puzzle, theorem: I) -> SolverResult<Solver<'a>>
        where I: Iterator<Item = Theorem>
    {
        let mut sum_of_hint = 0;
        for p in puzzle.points() {
            if let Some(n) = puzzle.hint(p) {
                sum_of_hint += n as u32;
            }
        }

        let mut side_map = SideMap::from(puzzle);
        let pool = try!(TheoremPool::new(theorem, puzzle, sum_of_hint, &mut side_map));

        Ok(Solver {
            puzzle: puzzle,
            sum_of_hint: sum_of_hint,
            theorem_pool: pool,
            side_map: side_map,
            connect_map: None,
            conn_analysis_revision: None,
        })
    }

    pub fn revision(&self) -> u32 {
        self.side_map.revision()
    }
    pub fn all_filled(&self) -> bool {
        self.side_map.all_filled()
    }

    pub fn get_side(&mut self, p: CellId) -> State<Side> {
        self.side_map.get_side(p)
    }
    pub fn set_inside(&mut self, p: CellId) -> bool {
        self.side_map.set_inside(p)
    }
    pub fn set_outside(&mut self, p: CellId) -> bool {
        self.side_map.set_outside(p)
    }

    pub fn validate_result(&mut self) -> SolverResult<()> {
        try!(self.sync_connection());
        if self.connect_map().count_area() != 2 {
            return Err(Error::invalid_board());
        }
        Ok(())
    }

    pub fn get_unknown_points(&mut self) -> Vec<CellId> {
        let mut pts = vec![];

        let mut conn_map = self.connect_map();

        for i in 0..conn_map.cell_len() {
            let p = CellId::new(i);
            let a = conn_map.get(p);
            if a.coord() == p && a.side() == State::Unknown {
                pts.push((p, a.unknown_edge().len()));
            }
        }

        pts.sort_by(|a, b| a.1.cmp(&b.1));
        pts.into_iter().map(|pair| pair.0).collect()
    }

    pub fn apply_all_theorem(&mut self) -> SolverResult<()> {
        self.theorem_pool.apply_all(&mut self.side_map)
    }
    pub fn connect_analysis(&mut self) -> SolverResult<()> {
        self.create_connect_map();
        ::step::connect_analysis::run(&mut self.side_map,
                                      self.connect_map.as_mut().unwrap(),
                                      &mut self.conn_analysis_revision)
    }

    pub fn mark_common(&mut self, s0: &mut Solver, s1: &mut Solver) {
        for i in 0..self.puzzle.cell_len() {
            let p = CellId::new(i);
            if let State::Fixed(side) = s0.side_map.get_side(p) {
                if s1.side_map.get_side(p) == State::Fixed(side) {
                    self.side_map.set_side(p, side);
                }
            }
        }

        for i in 0..(self.puzzle.cell_len() - 1) {
            let p0 = CellId::new(i);
            let p1 = CellId::new(i + 1);
            if let State::Fixed(side) = s0.side_map.get_edge(p0, p1) {
                if s1.side_map.get_edge(p0, p1) == State::Fixed(side) {
                    self.side_map.set_edge(p0, p1, side);
                }
            }
        }

        for i in 0..(self.puzzle.cell_len() - (self.puzzle.column() as usize)) {
            let p0 = CellId::new(i);
            let p1 = CellId::new(i + (self.puzzle.column() as usize));
            if let State::Fixed(side) = s0.side_map.get_edge(p0, p1) {
                if s1.side_map.get_edge(p0, p1) == State::Fixed(side) {
                    self.side_map.set_edge(p0, p1, side);
                }
            }
        }
    }

    fn create_connect_map(&mut self) {
        if self.connect_map.is_none() {
            let conn_map = ConnectMap::new(self.puzzle, &mut self.side_map);
            self.connect_map = Some(conn_map);
        }
    }
    fn connect_map(&mut self) -> &mut ConnectMap {
        self.create_connect_map();
        self.connect_map.as_mut().unwrap()
    }
    fn sync_connection(&mut self) -> SolverResult<()> {
        self.create_connect_map();
        self.connect_map.as_mut().unwrap().sync(&mut self.side_map)
    }

    // Utility function for debug.
    // pub fn dump(&self) -> String {
    //     if let Ok(result) = self.side_map.clone().into() {
    //         format!("{}", result)
    //     } else {
    //         format!("Error!")
    //     }
    // }
}

impl<'a> Into<SolverResult<Puzzle>> for Solver<'a> {
    fn into(mut self) -> SolverResult<Puzzle> {
        let mut puzzle = self.puzzle.clone();
        try!(self.side_map.complete_puzzle(&mut puzzle));
        Ok(puzzle)
    }
}
