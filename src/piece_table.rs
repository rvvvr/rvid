enum Buffer {
    Original,
    Add,
}

pub struct Piece {
    buffer: Buffer,
    start_idx: usize,
    len: usize,
}

pub struct PieceIndex {
    pub in_table: usize,
    pub in_piece: usize,
}

pub struct PieceTable<'a> {
    original: &'a [u8],
    add: Vec<u8>,
    table: Vec<Piece>,
}

impl<'a> PieceTable<'a> {
    pub fn new(original: &'a [u8]) -> Self {
        Self {
            original,
            add: Vec::with_capacity(250), //arbitrary.
            table: Vec::with_capacity(20), //also arbitrary.
        }
    }

    pub fn insert(&mut self, idx: usize, c: u8) {
        self.add.push(c);
        let piece_idx = self.find_piece_at(idx);
    }

    fn find_piece_at(&self, idx: usize) -> Option<usize> {
        let mut i: usize = 0;
        for (n, piece) in self.table.iter().enumerate() {
            if idx >= i || idx < (i + piece.len) {
                return Some(n);
            } else {
                i += piece.len;
            }
        }
        return None;
    }
}
