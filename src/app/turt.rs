/*
rfunge – a Funge-98 interpreter
Copyright © 2021 Thomas Jollans

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};

use rfunge::interpreter::fingerprints::TURT::{calc_bounds, Colour, Dot, Line, TurtleDisplay};

pub struct LocalTurtDisplay;

fn css_colour(clr: Colour) -> String {
    format!("rgb({}, {}, {})", clr.r, clr.g, clr.b)
}

impl TurtleDisplay for LocalTurtDisplay {
    fn display(&mut self, _show: bool) {}
    fn display_visible(&self) -> bool {
        false
    }
    fn draw(&mut self, _background: Option<Colour>, _lines: &[Line], _dots: &[Dot]) {}
    fn print(&mut self, background: Option<Colour>, lines: &[Line], dots: &[Dot]) {
        // craft an SVG
        // figure out the bounding box
        let (topleft, bottomright) = calc_bounds(lines.iter(), dots.iter());
        let x0 = topleft.x as f64 - 0.5;
        let y0 = topleft.y as f64 - 0.5;
        let width = bottomright.x - topleft.x + 1;
        let height = bottomright.y - topleft.y + 1;
        let mut svg = r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_owned();
        svg.push_str(&format!(
            r#"<svg viewBox="{} {} {} {}" xmlns="http://www.w3.org/2000/svg" stroke-linecap="square" stroke-width="1">"#,
            x0, y0, width, height));
        // Add the background
        if let Some(clr) = background {
            svg.push_str(&format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
                x0,
                y0,
                width,
                height,
                css_colour(clr)
            ))
        }
        // Add the lines
        for line in lines {
            svg.push_str(&format!(
                r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{}"/>"#,
                line.from.x,
                line.from.y,
                line.to.x,
                line.to.y,
                css_colour(line.colour)
            ));
        }
        // Add the dots
        for dot in dots {
            svg.push_str(&format!(
                r#"<circle cx="{}" cy="{}" r="0.5" fill="{}"/>"#,
                dot.pos.x,
                dot.pos.y,
                css_colour(dot.colour)
            ));
        }
        // Close tag
        svg.push_str("</svg>\n");

        // Write to file
        let mut fn_idx = 1;
        let mut fname = "rfunge_TURT_image.svg".to_owned();
        loop {
            // Create a new file!
            match OpenOptions::new().write(true).create_new(true).open(&fname) {
                Ok(mut out_f) => {
                    eprintln!("Writing TURT image to {}", fname);
                    out_f.write_all(svg.as_bytes()).unwrap_or_else(|e| {
                        eprintln!("Error writing to file {} ({:?})", fname, e);
                    });
                    break;
                }
                Err(e) => {
                    match e.kind() {
                        ErrorKind::AlreadyExists => {
                            // Try another filename
                            fn_idx = fn_idx + 1;
                            fname = format!("rfunge_TURT_image-{}.svg", fn_idx);
                            continue;
                        }
                        _ => {
                            eprintln!("Error opening file {} ({:?})", fname, e);
                            break;
                        }
                    }
                }
            }
        }
    }
}
