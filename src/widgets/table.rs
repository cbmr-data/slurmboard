use std::{fmt::Display, marker::PhantomData};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Row, StatefulWidgetRef, Table, TableState},
};

use super::{misc::COLUMN_SPACING, RightScrollbar};

pub trait GenericTableState<C>
where
    C: Copy + Display + Sized,
{
    fn focus(&self) -> bool;

    fn nrows(&self) -> usize;
    // FIXME: Should return slice
    fn columns(&self) -> &[C];

    /// Returns the text object for a given row and column. The `constraint` value
    /// will either be a constant
    fn text<'a>(&self, constraint: &Constraint, row: usize, column: C) -> Text<'a>;
    /// Returns true if a column should grow to consume available space
    fn variable_width(&self, column: C) -> bool;

    /// Returns TableState object used by the actual table
    fn inner_state(&mut self) -> &mut TableState;
    /// Returns the currently selected item
    fn selected(&self) -> Option<usize>;
}

#[derive(Debug, Default)]
pub struct GenericTable<C, S>
where
    C: Copy + Display + Sized,
    S: GenericTableState<C>,
{
    c: PhantomData<C>,
    s: PhantomData<S>,
}

impl<C, S> GenericTable<C, S>
where
    C: Copy + Display + Sized,
    S: GenericTableState<C>,
{
    pub fn new() -> Self {
        Self {
            c: PhantomData,
            s: PhantomData,
        }
    }

    fn width(state: &S, column: C) -> Option<Constraint> {
        if state.variable_width(column) {
            None
        } else {
            // Dummy value
            let constraint = Constraint::Length(32);
            let mut width = column.to_string().chars().count();
            for row in 0..state.nrows() {
                width = state.text(&constraint, row, column).width().max(width);
            }

            Some(Constraint::Length(width as u16))
        }
    }

    fn constraints(state: &S, area: Rect) -> Vec<Constraint> {
        let widths = state
            .columns()
            .iter()
            .map(|c| Self::width(state, *c))
            .collect::<Vec<_>>();

        let variable_length_columns = widths.iter().filter(|v| v.is_none()).count() as u16;
        let fixed_column_width = widths
            .iter()
            .map(|v| v.map(constraint_length).unwrap_or_default())
            .sum::<u16>();

        let marker_width = 0;
        let spacing_width = (widths.len().saturating_sub(1)) as u16 * COLUMN_SPACING;
        let fixed_width = marker_width + spacing_width + fixed_column_width;
        let bar_width = area.width.saturating_sub(fixed_width) / variable_length_columns.max(1);

        widths
            .into_iter()
            .map(|v| v.unwrap_or(Constraint::Length(bar_width)))
            .collect()
    }
}

impl<C, S> StatefulWidgetRef for GenericTable<C, S>
where
    C: Copy + Display + Sized,
    S: GenericTableState<C>,
{
    type State = S;

    #[doc = " Draws the current state of the widget in the given buffer. That is the only method required"]
    #[doc = " to implement a custom stateful widget."]
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let columns = state.columns();
        let area = RightScrollbar::default()
            .header(1)
            .items(state.nrows())
            .selected(state.selected())
            .render(area, buf);

        let constraints = GenericTable::<C, S>::constraints(state, area);

        let mut rows = Vec::new();
        for idx in 0..state.nrows() {
            let mut row = Row::new(
                columns
                    .iter()
                    .enumerate()
                    .map(|(i, &c)| state.text(&constraints[i], idx, c)),
            );

            // Used instead of Table::highlight_style so that it doesn't override the style of individual
            // cells; this is required since Utilization bars use both fg and bg colors to draw fractions.
            if state.selected() == Some(idx) {
                row = row.style(Style::default().bg(if state.focus() {
                    Color::Blue
                } else {
                    Color::LightBlue
                }));
            }

            rows.push(row);
        }

        let table = Table::new(rows, constraints)
            .column_spacing(COLUMN_SPACING)
            .header(Row::new(
                state.columns().iter().map(C::to_string).collect::<Vec<_>>(),
            ));

        StatefulWidgetRef::render_ref(&table, area, buf, &mut state.inner_state());
    }
}

fn constraint_length(c: Constraint) -> u16 {
    match c {
        Constraint::Min(v) | Constraint::Max(v) | Constraint::Length(v) => v,
        _ => unimplemented!(),
    }
}
