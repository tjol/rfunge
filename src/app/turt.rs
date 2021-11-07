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

#[cfg(feature = "turt-gui")]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};

#[cfg(feature = "turt-gui")]
use piston_window::{
    clear, Context, Ellipse, G2d, Line as PistonLine, PistonWindow, WindowSettings,
};

use rfunge::interpreter::fingerprints::TURT::{calc_bounds, Colour, Dot, Line, TurtleDisplay};

#[cfg(feature = "turt-gui")]
use super::env::CmdLineEnv;
#[cfg(feature = "turt-gui")]
use rfunge::interpreter::fingerprints::TURT::Point;
#[cfg(feature = "turt-gui")]
use rfunge::{Funge, Interpreter, ProgramResult, RunMode};

#[derive(Debug, Default)]
struct TurtImage {
    background: Option<Colour>,
    lines: Vec<Line>,
    dots: Vec<Dot>,
}

#[cfg(feature = "turt-gui")]
enum TurtGuiMsg {
    Finished,
    OpenDisplay,
    CloseDisplay,
}

#[cfg(feature = "turt-gui")]
#[derive(Debug, Default)]
pub struct LocalTurtDisplay {
    state: Arc<Mutex<TurtImage>>,
    msg_channel: Option<mpsc::Sender<TurtGuiMsg>>,
    display_active: Arc<AtomicBool>,
}

#[cfg(not(feature = "turt-gui"))]
#[derive(Debug, Default)]
pub struct LocalTurtDisplay;

impl LocalTurtDisplay {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(feature = "turt-gui")]
pub fn run_with_turt<InitFn, Interp>(make_interpreter: InitFn) -> ProgramResult
where
    InitFn: FnOnce() -> Interpreter<Interp::Idx, Interp::Space, Interp::Env> + Send + 'static,
    Interp: Funge<Env = CmdLineEnv> + 'static,
{
    let mut disp = LocalTurtDisplay::new();
    let disp_state = disp.state.clone();
    let disp_active = disp.display_active.clone();
    let (tx, rx) = mpsc::channel();
    let turt_tx = tx.clone();
    disp.msg_channel.replace(turt_tx);

    let worker_handle = std::thread::spawn(move || {
        let mut interpreter = make_interpreter();
        interpreter.env.as_mut().unwrap().init_turt(disp);
        let result = interpreter.run(RunMode::Run);
        tx.send(TurtGuiMsg::Finished).ok();
        result
    });

    let finish = || worker_handle.join().unwrap();

    // Wait for messages from the worker thread
    loop {
        match rx.recv() {
            Ok(TurtGuiMsg::Finished) => {
                return finish();
            }
            Ok(TurtGuiMsg::OpenDisplay) => {}
            Ok(TurtGuiMsg::CloseDisplay) => {
                // Display is not open right now
                continue;
            }
            Err(_) => {
                panic!("Unexpected RecvError");
            }
        }
        // Open the TURT display
        if let Ok(mut window) = WindowSettings::new("RFunge TURT", (600, 600))
            .exit_on_esc(true)
            .build::<PistonWindow>()
        {
            disp_active.store(true, Ordering::Release);
            while let Some(ev) = window.next() {
                window.draw_2d(&ev, |c, g, _d| {
                    if let Ok(img) = disp_state.lock() {
                        draw_turt(&c, g, &*img);
                    }
                });
                // Check for messages from the worker thread
                match rx.try_recv() {
                    Ok(TurtGuiMsg::CloseDisplay) => {
                        break;
                    }
                    Ok(TurtGuiMsg::Finished) => {
                        return finish();
                    }
                    Err(mpsc::TryRecvError::Empty) | Ok(TurtGuiMsg::OpenDisplay) => {
                        // expected
                        continue;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        panic!("Unexpected disconnect!");
                    }
                }
            }
        }
        disp_active.store(false, Ordering::Release);
    }
}

#[cfg(feature = "turt-gui")]
fn draw_turt(c: &Context, g: &mut G2d, img: &TurtImage) {
    // Get the bounds
    const PADDING: i32 = 10;
    let (Point { x: x0, y: y0 }, Point { x: x1, y: y1 }) =
        calc_bounds(img.lines.iter(), img.dots.iter());
    let img_width = (x1 - x0 + PADDING) as f64;
    let img_height = (y1 - y0 + PADDING) as f64;
    // Get the window size
    let [width, height] = c.get_view_size();
    // How the image is scaled depends on the aspect ratio
    let window_aspect = width / height;
    let img_aspect = img_width / img_height;
    let xscale;
    let yscale;
    if window_aspect > img_aspect {
        // match height
        yscale = 2.0 / img_height;
        xscale = yscale / window_aspect;
    } else {
        // match width
        xscale = 2.0 / img_width;
        yscale = xscale * window_aspect;
    }
    // Build the transformation matrix
    let dx = xscale * (-(x0 + x1) as f64 / 2.0);
    let dy = yscale * ((y0 + y1) as f64 / 2.0);
    let transform = [[xscale, 0.0, dx], [0.0, -yscale, dy]];

    clear(img.background.map(gfx_colour).unwrap_or([1.0; 4]), g);
    for line in &img.lines {
        let line_style = PistonLine::new_round(gfx_colour(line.colour), 0.5);
        line_style.draw_from_to(
            gfx_pt(line.from),
            gfx_pt(line.to),
            &c.draw_state,
            transform,
            g,
        );
    }
    for dot in &img.dots {
        Ellipse::new(gfx_colour(dot.colour)).draw(
            [dot.pos.x as f64 - 0.5, dot.pos.y as f64 - 0.5, 1.0, 1.0],
            &c.draw_state,
            transform,
            g,
        );
    }
}

fn css_colour(clr: Colour) -> String {
    format!("rgb({}, {}, {})", clr.r, clr.g, clr.b)
}

#[cfg(feature = "turt-gui")]
fn gfx_colour(clr: Colour) -> [f32; 4] {
    [
        clr.r as f32 / 255.0,
        clr.g as f32 / 255.0,
        clr.b as f32 / 255.0,
        1.0,
    ]
}

#[cfg(feature = "turt-gui")]
fn gfx_pt(p: Point) -> [f64; 2] {
    [p.x as f64, p.y as f64]
}

impl TurtleDisplay for LocalTurtDisplay {
    #[cfg(not(feature = "turt-gui"))]
    fn display(&mut self, _show: bool) {}
    #[cfg(not(feature = "turt-gui"))]
    fn display_visible(&self) -> bool {
        false
    }
    #[cfg(not(feature = "turt-gui"))]
    fn draw(&mut self, _background: Option<Colour>, _lines: &[Line], _dots: &[Dot]) {}

    #[cfg(feature = "turt-gui")]
    fn display(&mut self, show: bool) {
        let currently_visible = self.display_visible();
        if show && !currently_visible {
            // Tell the main thread to show the GUI
            self.msg_channel
                .as_mut()
                .and_then(|tx| tx.send(TurtGuiMsg::OpenDisplay).ok());
            self.display_active.store(true, Ordering::Release);
        } else if !show && currently_visible {
            // Tell the main thread to close the GUI
            self.msg_channel
                .as_mut()
                .and_then(|tx| tx.send(TurtGuiMsg::CloseDisplay).ok());
        }
    }
    #[cfg(feature = "turt-gui")]
    fn display_visible(&self) -> bool {
        self.display_active.load(Ordering::Acquire)
    }
    #[cfg(feature = "turt-gui")]
    fn draw(&mut self, background: Option<Colour>, lines: &[Line], dots: &[Dot]) {
        if let Ok(mut img_state) = self.state.lock() {
            img_state.background = background;
            img_state.lines.clear();
            img_state.dots.clear();
            img_state.lines.extend_from_slice(lines);
            img_state.dots.extend_from_slice(dots);
        }
    }

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
            r#"<svg viewBox="{} {} {} {}" xmlns="http://www.w3.org/2000/svg" stroke-linecap="round" stroke-width="1">"#,
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
                            fn_idx += 1;
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
