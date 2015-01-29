use std::str::FromStr;
use std::num::SignedInt;
use board::{Edge, Hint};
use geom::{Point, Rotation, Move, Size,
           LEFT, UP, UCW90, UCW180, UCW270, H_FLIP, V_FLIP};
use util;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Pattern {
    Hint(Hint, Point),
    Edge(Edge, Point, Point)
}

impl Pattern {
    fn hint(h: Hint, p: Point) -> Pattern {
        Pattern::Hint(h, p).normalized()
    }
    fn cross(p0: Point, p1: Point) -> Pattern {
        Pattern::Edge(Edge::Cross, p0, p1).normalized()
    }
    fn line(p0: Point, p1: Point) -> Pattern {
        Pattern::Edge(Edge::Line, p0, p1).normalized()
    }

    fn normalized(self) -> Pattern {
        match self {
            Pattern::Edge(edge, p0, p1) if p1 < p0 => {
                Pattern::Edge(edge, p1, p0)
            }
            x => x
        }
    }

    fn rotate(self, rot: Rotation) -> Pattern {
        let o = Point(0, 0);
        match self {
            Pattern::Hint(h, p) => { Pattern::Hint(h, o + rot * (p - o)) }
            Pattern::Edge(e, p0, p1) => {
                Pattern::Edge(e, o + rot * (p0 - o), o + rot * (p1 - o))
            }
        }.normalized()
    }

    fn shift(self, d: Move) -> Pattern {
        match self {
            Pattern::Hint(h, p) => { Pattern::Hint(h, p + d) }
            Pattern::Edge(e, p0, p1) => {
                Pattern::Edge(e, p0 + d, p1 + d)
            }
        }.normalized()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Theorem {
    size: Size,
    matcher: Vec<Pattern>,
    result: Vec<Pattern>
}

impl Theorem {
    fn normalized(mut self) -> Theorem {
        self.matcher.sort();
        self.matcher.dedup();
        self.result.sort();
        self.result.dedup();
        self
    }

    fn rotate(self, rot: Rotation) -> Theorem {
        let Theorem { size, matcher, result } = self;
        let size = rot * Move(size.0, size.1);

        let mut d = Move(0, 0);
        if size.0 < 0 { d = d + Move(- size.0 - 1, 0); }
        if size.1 < 0 { d = d + Move(0, - size.1 - 1); }

        Theorem {
            size: Size(size.0.abs(), size.1.abs()),
            matcher: matcher.map_in_place(|x| x.rotate(rot).shift(d)),
            result: result.map_in_place(|x| x.rotate(rot).shift(d))
        }.normalized()
    }

    pub fn all_rotations(self) -> Vec<Theorem> {
        let Size(r, c) = self.size;
        let deg90  = self.clone().rotate(UCW90);
        let deg180 = self.clone().rotate(UCW180);
        let deg270 = self.clone().rotate(UCW270);
        let h_deg0   = self.clone().rotate(H_FLIP);
        let h_deg90  = h_deg0.clone().rotate(UCW90);
        let h_deg180 = h_deg0.clone().rotate(UCW180);
        let h_deg270 = h_deg0.clone().rotate(UCW270);
        let mut rots = vec![self.clone(), deg90, deg180, deg270,
                            h_deg0, h_deg90, h_deg180, h_deg270];
        rots.sort();
        // FIXME: Should reduce the elements that has different result but size
        //        and matcher are same.
        rots.dedup();

        rots
    }
}

impl FromStr for Theorem {
    fn from_str(s: &str) -> Option<Theorem> {
        let mut matcher_lines = vec![];
        let mut result_lines = vec![];

        let mut lines = s.lines()
            .map(|l| l.trim_matches('\n'))
            .filter(|s| !s.is_empty());

        for line in lines {
            let mut it = line.splitn(2, '!');
            if let Some(l) = it.next() {
                matcher_lines.push(l.chars().collect());
            } else {
                return None
            }

            if let Some(l) = it.next() {
                result_lines.push(l.chars().collect());
            } else {
                return None
            }
        }

        let (m_size, m_pat) = match parse_lines(&matcher_lines[]) {
            Some(x) => x, None => return None
        };
        let (r_size, mut r_pat) = match parse_lines(&result_lines[]) {
            Some(x) => x, None => return None
        };

        if m_size != r_size { return None }

        let mut idx = 0;
        for &p in m_pat.iter() {
            match r_pat[idx ..].iter().position(|&x| x == p) {
                Some(i) => {
                    idx += i;
                    let _ = r_pat.remove(idx);
                }
                None => { return None }
            }
        }

        return Some(Theorem { size: m_size, matcher: m_pat, result: r_pat });

        fn parse_lines(lines: &[Vec<char>]) -> Option<(Size, Vec<Pattern>)> {
            use util::{VEdges, HEdges, Cells};

            let (rows, cols) = match util::find_lattice(lines) {
                Some(x) => x, None => return None
            };

            if rows.len() <= 1 { return None }
            if cols.len() <= 1 { return None }

            let size = Size((rows.len() - 1) as i32, (cols.len() - 1) as i32);

            let mut pat = vec![];

            for (p, s) in VEdges::new(lines, &rows[], &cols[]) {
                if s.is_empty() {
                    continue
                }
                if s.chars().all(|c| c == 'x') {
                    pat.push(Pattern::cross(p + LEFT, p));
                    continue
                }
                if s.chars().all(|c| c == '|') {
                    pat.push(Pattern::line(p + LEFT, p));
                    continue
                }
            }

            for (p, s) in HEdges::new(lines, &rows[], &cols[]) {
                if s.is_empty() {
                    continue
                }
                if s.chars().all(|c| c == 'x') {
                    pat.push(Pattern::cross(p + UP, p));
                    continue
                }
                if s.chars().all(|c| c == '-') {
                    pat.push(Pattern::line(p + UP, p));
                    continue
                }
            }

            let mut pairs: Vec<(char, Vec<Point>, Vec<Point>)> = vec![];

            for (p, s) in Cells::new(lines, &rows[], &cols[]) {
                for c in s.trim_matches(' ').chars() {
                    match c {
                        '0' => { pat.push(Pattern::hint(Some(0), p)); }
                        '1' => { pat.push(Pattern::hint(Some(1), p)); }
                        '2' => { pat.push(Pattern::hint(Some(2), p)); }
                        '3' => { pat.push(Pattern::hint(Some(3), p)); }
                        _ if c.is_alphabetic() => {
                            let key = c.to_lowercase();
                            match pairs.iter().position(|&(k, _, _)| k == key) {
                                Some(idx) => {
                                    if c.is_lowercase() {
                                        pairs[idx].1.push(p);
                                    } else {
                                        pairs[idx].2.push(p);
                                    }
                                }
                                None => {
                                    let (lower, upper) = if c.is_lowercase() {
                                        (vec![p], vec![])
                                    } else {
                                        (vec![], vec![p])
                                    };
                                    pairs.push((key, lower, upper));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            for &(_, ref ps0, ref ps1) in pairs.iter() {
                if !ps0.is_empty() && !ps1.is_empty() {
                    pat.push(Pattern::line(ps0[0], ps1[0]));
                }

                if ps0.len() > 0 {
                    for &p in ps0[1 ..].iter() {
                        pat.push(Pattern::cross(ps0[0], p));
                    }
                }
                if ps1.len() > 0 {
                    for &p in ps1[1 ..].iter() {
                        pat.push(Pattern::cross(ps1[0], p));
                    }
                }
            }

            pat.sort();
            pat.dedup();
            Some((size, pat))
        }
    }
}

#[cfg(test)]
mod tests {
    use geom::{Point, Size, Move, UCW0, UCW90, UCW180, UCW270, H_FLIP, V_FLIP};
    use board::Edge;
    use super::{Pattern, Theorem};

    #[test]
    fn parse() {
        fn check(size: Size, matcher: Vec<Pattern>, result: Vec<Pattern>,
                 input: &str)
        {
            let mut matcher = matcher.map_in_place(|p| p.normalized());
            let mut result = result.map_in_place(|p| p.normalized());
            matcher.sort();
            matcher.dedup();
            result.sort();
            result.dedup();
            assert_eq!(Some(Theorem { size: size, matcher: matcher, result: result }),
                       input.parse::<Theorem>())
        }

        check(Size(1, 1),
              vec![Pattern::hint(Some(0), Point(0, 0))],
              vec![Pattern::cross(Point(0, 0), Point(0, -1)),
                   Pattern::cross(Point(0, 0), Point(0, 1)),
                   Pattern::cross(Point(0, 0), Point(-1, 0)),
                   Pattern::cross(Point(0, 0), Point(1, 0))],"
+ + ! +x+
 0  ! x0x
+ + ! +x+
");
        check(Size(3, 3),
              vec![Pattern::hint(Some(0), Point(1, 0)),
                   Pattern::hint(Some(3), Point(1, 1))],
              vec![Pattern::cross(Point(1, 0), Point(1, -1)),
                   Pattern::cross(Point(1, 0), Point(1, 1)),
                   Pattern::cross(Point(1, 0), Point(0, 0)),
                   Pattern::cross(Point(1, 0), Point(2, 0)),
                   Pattern::cross(Point(0, 1), Point(0, 2)),
                   Pattern::cross(Point(1, 2), Point(0, 2)),
                   Pattern::cross(Point(1, 2), Point(2, 2)),
                   Pattern::cross(Point(2, 1), Point(2, 2)),
                   Pattern::line(Point(0, 0), Point(0, 1)),
                   Pattern::line(Point(0, 1), Point(1, 1)),
                   Pattern::line(Point(1, 1), Point(1, 2)),
                   Pattern::line(Point(1, 1), Point(2, 1)),
                   Pattern::line(Point(2, 0), Point(2, 1))], "
+ + + + ! + + + +
        !   | x
+ + + + ! +x+-+x+
 0 3    ! x0x3|
+ + + + ! +x+-+x+
        !   | x
+ + + + ! + + + +
");
        check(Size(2, 2),
              vec![Pattern::hint(Some(1), Point(1, 1)),
                   Pattern::line(Point(1, 0), Point(0, 1))],
              vec![Pattern::cross(Point(1, 1), Point(1, 2)),
                   Pattern::cross(Point(1, 1), Point(2, 1))], "
+ + + ! + + +
   a  !    a
+ + + ! + + +
 A 1  !  A 1x
+ + + ! + +x+
");
        check(Size(3, 3),
              vec![Pattern::hint(Some(3), Point(1, 1)),
                   Pattern::cross(Point(1, 0), Point(0, 1))],
              vec![Pattern::cross(Point(0, 0), Point(0, 1)),
                   Pattern::cross(Point(0, 0), Point(1, 0)),
                   Pattern::line(Point(0, 1), Point(1, 1)),
                   Pattern::line(Point(1, 0), Point(1, 1)),
                   Pattern::line(Point(1, 2), Point(2, 1))], "
+ + + + ! + + + +
   a    !   xa
+ + + + ! +x+-+ +
 a 3    !  a|3 b
+ + + + ! + + + +
        !    B
+ + + + ! + + + +
");
    }

    #[test]
    fn rotate() {
        let deg0 = "
+ + + ! + + +
   a  !  bxa
+ + + ! +x+-+
 a 3  !  a|3
+ + + ! + + +
      !    B
+ + + ! + + +
".parse::<Theorem>().unwrap();

        let deg90 = "
+ + + + ! + + + +
 a 3    !  a|3 B
+ + + + ! +x+-+ +
   a    !  bxa
+ + + + ! + + + +
".parse::<Theorem>().unwrap();

        let deg180 = "
+ + + ! + + +
      !  B
+ + + ! + + +
 3 a  !  3|a
+ + + ! +-+x+
 a    !  axb
+ + + ! + + +
".parse::<Theorem>().unwrap();

        let deg270 = "
+ + + + ! + + + +
   a    !    axb
+ + + + ! + +-+x+
   3 a  !  B 3|a
+ + + + ! + + + +
".parse::<Theorem>().unwrap();

        let h_flip = "
+ + + ! + + +
 a    !  axb
+ + + ! +-+x+
 3 a  !  3|a
+ + + ! + + +
      !  B
+ + + ! + + +
".parse::<Theorem>().unwrap();

        let v_flip = "
+ + + ! + + +
      !    B
+ + + ! + + +
 a 3  !  a|3
+ + + ! +x+-+
   a  !  bxa
+ + + ! + + +
".parse::<Theorem>().unwrap();

        let Size(r, c) = deg0.size;
        assert_eq!(deg0.clone(), deg0.clone().rotate(UCW0));
        assert_eq!(deg90.clone(), deg0.clone().rotate(UCW90));
        assert_eq!(deg180.clone(), deg0.clone().rotate(UCW180));
        assert_eq!(deg270.clone(), deg0.clone().rotate(UCW270));
        assert_eq!(h_flip.clone(), deg0.clone().rotate(H_FLIP));
        assert_eq!(v_flip.clone(), deg0.clone().rotate(V_FLIP));
        assert_eq!(v_flip.clone(), h_flip.clone().rotate(UCW180));

        let mut rots = [deg0.clone(), deg90, deg180, deg270,
                        h_flip.clone(),
                        h_flip.clone().rotate(UCW90),
                        h_flip.clone().rotate(UCW180),
                        h_flip.clone().rotate(UCW270)];
        rots.sort();
        assert_eq!(rots, deg0.all_rotations());
    }

    #[test]
    fn all_rotations() {
        let theo = "
+ + ! +x+
 0  ! x0x
+ + ! +x+
".parse::<Theorem>().unwrap();
        let rots = theo.clone().all_rotations();
        assert_eq!([theo], rots);
    }
}