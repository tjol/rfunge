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
use glutin::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder, WindowedContext,
};

// #[cfg(feature = "turt-gui")]
// use shader_version::OpenGL;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TurtGuiMsg {
    Finished,
    OpenDisplay,
    CloseDisplay,
    Redraw,
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

#[cfg(feature = "turt-gui")]
struct TurtWindowState {
    pub wnd_ctx: WindowedContext<glutin::PossiblyCurrent>,
    pub canvas: femtovg::Canvas<femtovg::renderer::OpenGl>,
}

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
            Ok(TurtGuiMsg::OpenDisplay) => {
                break;
            }
            Ok(_) => {
                // Display is not open right now
                continue;
            }
            Err(_) => {
                panic!("Unexpected RecvError");
            }
        }
    }

    // We have been asked to open a TURT display!
    // create a winit event loop
    let event_loop = EventLoop::with_user_event();
    let event_loop_proxy = event_loop.create_proxy();
    // Forward messages into the event loop as user events
    std::thread::spawn(move || loop {
        match rx.recv() {
            Ok(msg) => {
                event_loop_proxy.send_event(msg).ok();
                if msg == TurtGuiMsg::Finished {
                    return;
                }
            }
            Err(_) => {
                eprintln!("[Guru tempted to meditate]");
            }
        }
    });

    let event_loop_proxy = event_loop.create_proxy();

    // Inject an initial command into the event loop (the one we just got: open)
    event_loop_proxy
        .send_event(TurtGuiMsg::OpenDisplay)
        .unwrap();

    let mut window_state = None;

    // Run the loop
    event_loop.run(move |evt, el, control_flow| {
        *control_flow = ControlFlow::Wait;
        match evt {
            Event::UserEvent(TurtGuiMsg::OpenDisplay) => {
                let wb = WindowBuilder::new()
                    .with_title("RFunge TURT")
                    .with_inner_size(LogicalSize::new(400., 400.));
                // TODO graceful failure
                let wc = ContextBuilder::new().build_windowed(wb, el).unwrap();
                let wnd_ctx = unsafe { wc.make_current() }.unwrap();
                // Create the FemtoVG renderer and canvas
                use femtovg::renderer::OpenGl;
                // let renderer = OpenGl::new_from_glutin_context(&wnd_ctx).unwrap();
                let renderer = OpenGl::new(|s| wnd_ctx.get_proc_address(s) as *const _).unwrap();
                let canvas = femtovg::Canvas::new(renderer).unwrap();
                // Store the window-related stuff in the state variable
                window_state = Some(TurtWindowState { wnd_ctx, canvas });
                // Arrange for a redraw
                event_loop_proxy.send_event(TurtGuiMsg::Redraw).unwrap();
                disp_active.store(true, Ordering::Release);
            }
            Event::UserEvent(TurtGuiMsg::CloseDisplay) => {
                window_state = None;
                disp_active.store(false, Ordering::Release);
            }
            Event::UserEvent(TurtGuiMsg::Finished) => {
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(TurtGuiMsg::Redraw) => {
                if let Some(ws) = window_state.as_ref() {
                    ws.wnd_ctx.window().request_redraw();
                }
            }
            Event::RedrawRequested(_) => {
                if let Some(ws) = window_state.as_mut() {
                    let dpi_factor = ws.wnd_ctx.window().scale_factor();
                    let size = ws.wnd_ctx.window().inner_size();
                    // println!("dpi {:?}", dpi_factor);
                    ws.canvas
                        .set_size(size.width as u32, size.height as u32, dpi_factor as f32);
                    if let Ok(img) = disp_state.lock() {
                        draw_turt(&mut ws.canvas, &img);
                    }
                    ws.canvas.flush();
                    ws.wnd_ctx.swap_buffers().unwrap();
                }
            }
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    if let Some(ws) = window_state.as_mut() {
                        ws.wnd_ctx.resize(*physical_size);
                    }
                }
                WindowEvent::CloseRequested => {
                    event_loop_proxy
                        .send_event(TurtGuiMsg::CloseDisplay)
                        .unwrap();
                }
                _ => {}
            },
            _ => {}
        }
    });
}

#[cfg(feature = "turt-gui")]
fn draw_turt<R: femtovg::Renderer>(c: &mut femtovg::Canvas<R>, img: &TurtImage) {
    use femtovg::{Color, LineCap, Paint, Path};

    let width = c.width();
    let height = c.height();
    c.reset_transform();

    c.clear_rect(
        0,
        0,
        width as u32,
        height as u32,
        img.background
            .map(fvg_colour)
            .unwrap_or_else(|| Color::rgb(0xff, 0xff, 0xff)),
    );

    // println!("Cleared w {} h {}", width, height);

    // Figure out the right transformation
    const PADDING: i32 = 10;
    let (Point { x: x0, y: y0 }, Point { x: x1, y: y1 }) =
        calc_bounds(img.lines.iter(), img.dots.iter());
    let img_width = (x1 - x0 + PADDING) as f32;
    let img_height = (y1 - y0 + PADDING) as f32;

    // How the image is scaled depends on the aspect ratio
    let window_aspect = width / height;
    let img_aspect = img_width / img_height;
    let scale = if window_aspect > img_aspect {
        // match height
        height / img_height
    } else {
        // match width
        width / img_width
    };
    c.scale(scale, scale);

    // Centre the image
    let dx = width / scale / 2.0 - (x0 + x1) as f32 / 2.0;
    let dy = height / scale / 2.0 - (y0 + y1) as f32 / 2.0;
    c.translate(dx, dy);

    for line in &img.lines {
        let mut paint = Paint::color(fvg_colour(line.colour));
        paint.set_line_cap(LineCap::Round);
        paint.set_line_width(1.0);

        let mut path = Path::new();
        path.move_to(line.from.x as f32, line.from.y as f32);
        path.line_to(line.to.x as f32, line.to.y as f32);
        c.stroke_path(&mut path, paint);
    }

    for dot in &img.dots {
        let paint = Paint::color(fvg_colour(dot.colour));
        let mut path = Path::new();
        path.circle(dot.pos.x as f32, dot.pos.y as f32, 0.5);
        c.fill_path(&mut path, paint);
    }
}

fn css_colour(clr: Colour) -> String {
    format!("rgb({}, {}, {})", clr.r, clr.g, clr.b)
}

#[cfg(feature = "turt-gui")]
fn fvg_colour(clr: Colour) -> femtovg::Color {
    femtovg::Color::rgb(clr.r, clr.g, clr.b)
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
