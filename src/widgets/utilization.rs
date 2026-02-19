use std::iter::Sum;

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

const BARS: [&str; 8] = ["█", "▉", "▊", "▋", "▌", "▍", "▎", "▏"];

#[derive(Debug, Default)]
pub struct Utilization {
    /// Actual utilization; should normally be less than reserved, but may exceed it
    /// due to resource usage by system processes
    pub utilized: f64,
    /// Amount of resources allocated to users. Must be less or equal to capacity
    pub allocated: f64,
    /// Amount of resources "blocked" due to (over)allocation of linked resources;
    /// CPUs may for example be "blocked" due to (over)allocation of RAM, unless a
    /// user explicitly asks for less than the default amount of RAM per CPU.
    pub blocked: f64,
    /// Amount of resources unavailable for other reasons (node down)
    pub unavailable: f64,
    /// Total amount of resources available
    pub capacity: f64,
}

impl Utilization {
    pub fn available(&self) -> f64 {
        self.capacity - (self.allocated + self.blocked + self.unavailable)
    }

    pub fn to_line<'a>(self, length: u16) -> Line<'a> {
        assert!(self.allocated + self.unavailable <= self.capacity);

        let mut spans = Vec::new();
        if length > 0 && self.capacity > 0.0 {
            // Total number of chars appended
            let mut chars = 0usize;

            // CPUs available to the user
            let available = self.capacity - self.unavailable;
            // List of segments by their end-point and their colors
            let segments = [
                // Utilization may spike above resources available to users/Slurm,
                // but it doesn't make sense to show utilization beyond the resources
                // actually available to the users
                (self.utilized.min(available), Color::Green),
                // Allocated but unutilized resources
                (self.allocated, Color::Yellow),
                // Resources blocked to to allocation of linked resources
                (self.blocked, Color::LightMagenta),
                // Unblocked, unallocated resources
                (available, Color::DarkGray),
                // Unavailable resources
                (self.capacity, Color::Black),
            ];

            let mut last_end = 0.0;
            let mut last_color = Color::Green;

            for (end, color) in segments {
                let end = (end / self.capacity) * length as f64;

                // Utilization in particular may exceed subsequent bars
                if end > last_end {
                    // Bars will typically partially overlap the trailing character
                    let remainder = last_end - last_end.floor();
                    assert!((0.0..1.0).contains(&remainder));
                    let fraction = (remainder * 8.0) as isize - 1;

                    if fraction > 0 {
                        let style = style(last_color, color);
                        spans.push(Span::styled(BARS[fraction as usize], style));
                        last_end += 1.0 - remainder;
                        chars += 1;
                    } else {
                        // Truncate the last segment, since the remainder too short to render
                        last_end -= remainder;
                    }

                    // Depending on the remaining fraction, end may now be less than last_end
                    if end > last_end {
                        let whole = (end - last_end) as usize;
                        if whole > 0 {
                            let style = style(color, color);
                            spans.push(Span::styled(BARS[0].repeat(whole), style));
                            chars += whole;
                        }

                        last_end = end;
                        last_color = color;
                    }
                }
            }

            let remainder = (length as usize).saturating_sub(chars);
            if remainder > 0 {
                let style = style(last_color, last_color);
                spans.push(Span::styled(BARS[0].repeat(remainder), style));
            }
        }

        Line::from(spans)
    }
}

/// Implements the sum operator for Utilization objects
/// This is used for generating partition overviews
impl Sum for Utilization {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut result = Utilization::default();
        for it in iter {
            result.utilized += it.utilized;
            result.allocated += it.allocated;
            result.blocked += it.blocked;
            result.unavailable += it.unavailable;
            result.capacity += it.capacity;
        }
        result
    }
}

fn style(fg: Color, bg: Color) -> Style {
    Style::reset().fg(fg).bg(bg)
}
