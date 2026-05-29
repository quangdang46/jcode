use ftui::Frame;
use ftui_core::geometry::Rect;
use ftui_render::cell::PackedRgba;
use ftui_style::{Color, Style};
use ftui_text::text::Line;
use ftui_widgets::Widget;
use ftui_widgets::block::Alignment;
use ftui_widgets::block::Block;
use ftui_widgets::borders::Borders;
use ftui_widgets::paragraph::Paragraph;

pub fn clear_area(frame: &mut Frame, area: Rect) {
    for x in area.left()..area.right() {
        for y in area.top()..area.bottom() {
            if let Some(cell) = frame.buffer.get_mut(x, y) {
                *cell = ftui_render::cell::Cell::default();
            }
        }
    }
}

pub fn left_aligned_content_inset(width: u16, centered: bool) -> u16 {
    if centered || width <= 1 { 0 } else { 1 }
}

pub fn centered_content_block_width(width: u16, max_width: usize) -> usize {
    (width as usize).min(max_width).max(1)
}

pub fn left_pad_lines_to_block_width(
    _lines: &mut [Line<'static>],
    _width: u16,
    _block_width: usize,
) {
    todo!("ftui Line API differs - spans field is private, no alignment field")
}

const RIGHT_RAIL_HEADER_HEIGHT: u16 = 1;

pub fn right_rail_border_style(focused: bool, focus_color: Color, dim_color: Color) -> Style {
    let border_color = if focused { focus_color } else { dim_color };
    let rgb = border_color.to_rgb();
    Style::new().fg(PackedRgba::rgb(rgb.r, rgb.g, rgb.b))
}

fn right_rail_inner(area: Rect) -> Rect {
    Block::default().borders(Borders::LEFT).inner(area)
}

fn right_rail_content_area(area: Rect) -> Option<Rect> {
    let inner = right_rail_inner(area);
    if inner.width == 0 || inner.height <= RIGHT_RAIL_HEADER_HEIGHT {
        return None;
    }

    Some(Rect {
        x: inner.x,
        y: inner.y + RIGHT_RAIL_HEADER_HEIGHT,
        width: inner.width,
        height: inner.height - RIGHT_RAIL_HEADER_HEIGHT,
    })
}

pub fn draw_right_rail_chrome(
    frame: &mut Frame,
    area: Rect,
    title: Line<'static>,
    border_style: Style,
) -> Option<Rect> {
    let inner = right_rail_inner(area);
    let content_area = right_rail_content_area(area)?;

    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(border_style);
    block.render(area, frame);
    Paragraph::new(ftui_text::Text::from(title)).render(
        Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: RIGHT_RAIL_HEADER_HEIGHT,
        },
        frame,
    );

    Some(content_area)
}

pub fn align_if_unset(_line: Line<'static>, _align: Alignment) -> Line<'static> {
    todo!("ftui Line has no alignment field - paragraph-level alignment only")
}
