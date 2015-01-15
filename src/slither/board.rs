use std::{cmp, iter};
use std::io::{IoResult, BufferedReader};
use std::ops::{Index, IndexMut};

use geom::{Geom, Point, Size, Matrix};

pub type Hint = Option<u8>;
#[derive(Copy, Clone, Show, Eq, PartialEq)]
pub enum Side { In, Out }
#[derive(Copy, Clone, Show, Eq, PartialEq)]
pub enum Edge { Line, Cross }

#[derive(Clone, Show)]
pub struct Board {
    size: Size,
    hint: Matrix<Hint>,
    side: Matrix<Option<Side>>,
    edge_v: Matrix<Option<Edge>>,
    edge_h: Matrix<Option<Edge>>
}

impl Board {
    pub fn new(size: Size) -> Board {
        assert!(size.0 > 0 && size.1 > 0);
        let hint = Matrix::new_empty(size, None, None);
        let side = Matrix::new_empty(size, Some(Side::Out), None);
        let edge_v = Matrix::new_empty(Size(size.0, size.1 + 1), Some(Edge::Cross), None);
        let edge_h = Matrix::new_empty(Size(size.0 + 1, size.1), Some(Edge::Cross), None);
        Board { size: size, hint: hint, side: side, edge_v: edge_v, edge_h: edge_h }
    }

    fn with_data(size: Size, hint: Vec<Hint>, side: Vec<Option<Side>>,
                 edge_v: Vec<Option<Edge>>, edge_h: Vec<Option<Edge>>) -> Board {
        assert!(size.0 > 0 && size.1 > 0);
        let hint = Matrix::new(size, None, hint);
        let side = Matrix::new(size, Some(Side::Out), side);
        let edge_v = Matrix::new(Size(size.0, size.1 + 1), Some(Edge::Cross), edge_v);
        let edge_h = Matrix::new(Size(size.0 + 1, size.1), Some(Edge::Cross), edge_h);
        Board { size: size, hint: hint, side: side, edge_v: edge_v, edge_h: edge_h }
    }

    pub fn from_reader<R: Reader>(reader: R) -> IoResult<Board> {
        let mut br = BufferedReader::new(reader);

        let mut column = 0;
        let mut mat = vec![];
        for line in br.lines() {
            let row = try!(line).trim_matches('\n').chars().map(|c| {
                match c {
                    '0' => Some(0),
                    '1' => Some(1),
                    '2' => Some(2),
                    '3' => Some(3),
                    _   => None
                }
            }).collect::<Vec<_>>();

            column = cmp::max(column, row.len());
            mat.push(row);
        }
        for row in mat.iter_mut() {
            let len = row.len();
            if len < column {
                row.extend(iter::repeat(None).take(column - len));
            }
        }
        let row = mat.len();
        let size = Size(row as i32, column as i32);

        let side = iter::repeat(None).take(row * column).collect();
        let edge_v = iter::repeat(None).take(row * (column + 1)).collect();
        let edge_h = iter::repeat(None).take((row + 1) * column).collect();
        Ok(Board::with_data(size, mat.concat(), side, edge_v, edge_h))
    }

    pub fn hint(&self) -> &Matrix<Hint> { &self.hint }
    pub fn side(&self) -> &Matrix<Option<Side>> { &self.side }
    pub fn edge_h(&self) -> &Matrix<Option<Edge>> { &self.edge_h }
    pub fn edge_v(&self) -> &Matrix<Option<Edge>> { &self.edge_v }

    pub fn hint_mut(&mut self) -> &mut Matrix<Hint> { &mut self.hint }
    pub fn side_mut(&mut self) -> &mut Matrix<Option<Side>> { &mut self.side }
    pub fn edge_h_mut(&mut self) -> &mut Matrix<Option<Edge>> { &mut self.edge_h }
    pub fn edge_v_mut(&mut self) -> &mut Matrix<Option<Edge>> { &mut self.edge_v }
}

impl Geom for Board {
    fn size(&self) -> Size { self.size }
}

impl Index<Point> for Board {
    type Output = Hint;

    fn index(&self, p: &Point) -> &Hint {
        &self.hint[*p]
    }
}

impl IndexMut<Point> for Board {
    type Output = Hint;

    fn index_mut(&mut self, p: &Point) -> &mut Hint {
        &mut self.hint[*p]
    }
}

#[cfg(test)]
mod tests {
    use super::Board;
    use geom::{Geom, Size, Point};

    #[test]
    fn from_reader() {
        let input = "123   

345
";
        let hint = Board::from_reader(input.as_bytes()).unwrap();
        assert_eq!(Size(3, 6), hint.size());
        assert_eq!(Some(1), hint[Point(0, 0)]);
        assert_eq!(Some(2), hint[Point(0, 1)]);
        assert_eq!(Some(3), hint[Point(0, 2)]);
        assert_eq!(None, hint[Point(0, 3)]);
        assert_eq!(None, hint[Point(0, 4)]);
        assert_eq!(None, hint[Point(0, 5)]);
        assert_eq!(None, hint[Point(1, 0)]);
        assert_eq!(None, hint[Point(1, 1)]);
        assert_eq!(None, hint[Point(1, 2)]);
        assert_eq!(None, hint[Point(1, 3)]);
        assert_eq!(None, hint[Point(1, 4)]);
        assert_eq!(None, hint[Point(1, 5)]);
        assert_eq!(Some(3), hint[Point(2, 0)]);
        assert_eq!(None, hint[Point(2, 1)]);
        assert_eq!(None, hint[Point(2, 2)]);
        assert_eq!(None, hint[Point(2, 3)]);
        assert_eq!(None, hint[Point(2, 4)]);
        assert_eq!(None, hint[Point(2, 5)]);
    }
}