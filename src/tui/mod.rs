pub enum Dimension {
    Exact(u32),
    Filling,
    Proportion(f32),
}

pub struct BorderSet {
    pub top_left: char,
    pub top: char,
    pub top_right: char,
    pub left: char,
    pub right: char,
    pub bottom_left: char,
    pub bottom: char,
    pub bottom_right: char,
}

impl BorderSet {
    pub fn new(top_left: char,    
               top: char,
               top_right: char,
               left: char,
               right: char,
               bottom_left: char,
               bottom: char,
               bottom_right: char) -> Self {
        Self {
            top_left, top, top_right, left, right, bottom_left, bottom, bottom_right,
        }
    }
}

impl Default for BorderSet {
    fn default() -> Self {
        Self {
            top_left: '╭', 
            top: '─', 
            top_right: '╮', 
            left: '│', 
            right: '│',
            bottom_left: '╯', 
            bottom: '─', 
            bottom_right: '╰', 
        }
    }
}

pub enum Border {
    Nopers,
    Bordered(BorderSet),
}

pub struct WindowPane {
    pub width: Dimension,
    pub height: Dimension,
    pub border: Border
}

impl WindowPane {
    
}

pub trait Pane {
    
}
