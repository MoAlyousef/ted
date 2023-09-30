#![allow(dead_code)]

use crate::state::STATE;
use fltk::{enums::*, prelude::*, *};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::{
    env,
    io::{Read, Write},
    mem, str, thread,
};

pub struct PPTerm {
    st: text::SimpleTerminal,
}

impl PPTerm {
    pub fn new() -> Self {
        let mut st = text::SimpleTerminal::default().with_id("term");
        // SimpleTerminal handles many common ansi escape sequence
        st.set_ansi(true);
        let pair = native_pty_system()
            .openpty(PtySize {
                cols: 80,
                rows: 24,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("Failed to create pty");

        let mut cmd = if cfg!(target_os = "windows") {
            CommandBuilder::new("cmd.exe")
        } else {
            env::set_var("TERM", "vt100");
            CommandBuilder::new("/bin/bash")
        };
        cmd.cwd(env::current_dir().unwrap());
        cmd.env("PATH", env::var("PATH").unwrap());

        let mut child = pair.slave.spawn_command(cmd).unwrap();
        let mut reader = pair.master.try_clone_reader().unwrap();
        let mut writer = pair.master.take_writer().unwrap();
        mem::forget(pair);

        #[cfg(not(windows))]
        {
            thread::spawn({
                let mut st = st.clone();
                move || {
                    let mut s = Vec::new();
                    while child.try_wait().is_ok() {
                        let mut msg = [0u8; 1024];
                        if let Ok(sz) = reader.read(&mut msg) {
                            let msg = &msg[0..sz];
                            s.extend_from_slice(&msg[0..sz]);
                            match str::from_utf8(&s) {
                                Ok(text) => {
                                    if text != "\x07" {
                                        st.append(text);
                                    }
                                    s.clear();
                                }
                                Err(z) => {
                                    let z = z.valid_up_to();
                                    st.append2(&msg[0..z]);
                                    s.extend_from_slice(&msg[z..]);
                                }
                            }
                            app::awake();
                        }
                        app::sleep(0.03);
                    }
                }
            });
        }

        #[cfg(windows)]
        {
            // windows quirk
            app::sleep(0.03);
            thread::spawn({
                let mut st = st.clone();
                move || {
                    // let mut s = Vec::new();
                    while child.try_wait().is_ok() {
                        let mut msg = [0u8; 1024];
                        if let Ok(sz) = reader.read(&mut msg) {
                            let msg = &msg[0..sz];
                            st.append2(msg);
                        }
                        app::sleep(0.03);
                    }
                }
            });
        }

        let mut cmd = String::new();
        st.handle(move |t, ev| match ev {
            Event::KeyDown => {
                let key = app::event_key();
                if key == Key::Up || key == Key::Down {
                    writer
                        .write_all(
                            STATE
                                .with(|s| s.cmds.cmds.last().unwrap().clone())
                                .as_bytes(),
                        )
                        .unwrap();
                    t.scroll(t.count_lines(0, t.buffer().unwrap().length(), true), 0);
                } else {
                    let txt = app::event_text();
                    if txt == "\r" {
                        STATE.with({
                            let cmd = cmd.clone();
                            move |s| s.cmds.push(&cmd)
                        });
                        cmd.clear();
                    } else {
                        cmd.push_str(&txt);
                    }
                    writer.write_all(txt.as_bytes()).unwrap();
                }
                true
            }
            Event::KeyUp => {
                if app::event_key() == Key::Up {
                    t.scroll(t.count_lines(0, t.buffer().unwrap().length(), true), 0);
                    true
                } else {
                    false
                }
            }
            _ => false,
        });

        st.set_cursor_style(text::Cursor::Dim);
        // for a blinking cursors
        app::add_timeout3(1.0, {
            let mut st = st.clone();
            move |h| {
                if !st.has_focus() {
                    if st.cursor_style() == text::Cursor::Block {
                        st.set_cursor_style(text::Cursor::Dim);
                    } else {
                        st.set_cursor_style(text::Cursor::Block);
                    }
                } else {
                    st.set_cursor_style(text::Cursor::Block);
                }
                app::repeat_timeout3(1.0, h);
            }
        });

        Self { st }
    }
}

fltk::widget_extends!(PPTerm, text::SimpleTerminal, st);

pub struct XTerm {
    xterm_win: window::Window,
}

impl XTerm {
    pub fn new() -> Self {
        let mut xterm_win = window::Window::default().with_id("term");
        xterm_win.end();
        xterm_win.set_color(Color::Black);
        if crate::utils::is_session_x11() {}
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            if crate::utils::is_session_x11() {
                app::add_timeout3(0.1, {
                    let xterm_win = xterm_win.clone();
                    move |_h| {
                        let handle = xterm_win.raw_handle();
                        std::process::Command::new("xterm")
                            .args(&[
                                "-into",
                                &format!("{}", handle),
                                "-bg",
                                "black",
                                "-fg",
                                "white",
                            ])
                            .spawn()
                            .unwrap();
                    }
                });
            }
        }

        Self { xterm_win }
    }
}

fltk::widget_extends!(XTerm, window::Window, xterm_win);
