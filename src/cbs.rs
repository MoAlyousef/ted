use crate::state::STATE;
use fltk::{enums::*, prelude::*, *};
use std::{fs, path::PathBuf};

fn nfc_get_file(mode: dialog::NativeFileChooserType) -> PathBuf {
    let mut nfc = dialog::NativeFileChooser::new(mode);
    nfc.show();
    nfc.filename()
}

fn find() {
    let mut dlg: window::Window = app::widget_from_id("find").unwrap();
    let main_win = app::first_window().unwrap();
    dlg.resize(main_win.x() + main_win.w() - 300, dlg.y() + 30, 300, 50);
    dlg.show();
}

fn replace() {
    let mut dlg: window::Window = app::widget_from_id("replace").unwrap();
    let main_win = app::first_window().unwrap();
    dlg.resize(main_win.x() + main_win.w() - 300, dlg.y() + 30, 300, 80);
    dlg.show();
}

pub fn win_cb(_: &mut window::Window) {
    if app::event() == Event::Close {
        app::quit();
    }
}

pub fn editor_cb(_e: &mut text::TextEditor) {
    app::add_timeout3(0.01, |_| STATE.with(|s| s.was_modified(true)));
}

pub fn menu_cb(m: &mut impl MenuExt) {
    if let Ok(mpath) = m.item_pathname(None) {
        match mpath.as_str() {
            "&File/New...\t" => {
                STATE.with(|s| {
                    s.append(None);
                });
            }
            "&File/Open...\t" => {
                let c = nfc_get_file(dialog::NativeFileChooserType::BrowseFile);
                if c.exists() {
                    STATE.with(move |s| {
                        s.append(Some(c.canonicalize().unwrap()));
                    });
                }
            }
            "&File/Save\t" => {
                STATE.with(|s| {
                    if let Some(id) = s.current_id() {
                        let e = s.map.get(&id).unwrap();
                        let modified = e.modified;
                        if let Some(current_file) = e.current_file.as_ref() {
                            if modified && current_file.exists() {
                                fs::write(current_file, e.buf.text()).ok();
                                s.was_modified(false);
                            }
                        }
                    }
                });
            }
            "&File/Save as...\t" => {
                let c = nfc_get_file(dialog::NativeFileChooserType::BrowseSaveFile);
                if c.exists() {
                    STATE.with(move |s| {
                        if let Some(buf) = s.buf().as_ref() {
                            fs::write(&c, buf.text()).expect("Failed to write to file!");
                            s.was_modified(false);
                        }
                    });
                }
            }
            "&File/Save All\t" => {
                STATE.with(|s| {
                    for v in s.map.values_mut() {
                        if v.modified && v.current_file.as_ref().unwrap().exists() {
                            fs::write(v.current_file.as_ref().unwrap(), v.buf.text()).ok();
                            v.modified = true;
                        }
                    }
                });
            }
            "&File/Quit\t" => app::quit(),
            "&Edit/Undo\t" => STATE.with(|s| {
                if let Some(e) = s.current_editor() {
                    e.undo()
                }
            }),
            "&Edit/Redo\t" => STATE.with(|s| {
                if let Some(e) = s.current_editor() {
                    e.redo()
                }
            }),
            "&Edit/Cut\t" => STATE.with(|s| {
                if let Some(e) = s.current_editor() {
                    e.cut()
                }
            }),
            "&Edit/Copy\t" => STATE.with(|s| {
                if let Some(e) = s.current_editor() {
                    e.copy()
                }
            }),
            "&Edit/Paste\t" => STATE.with(|s| {
                if let Some(e) = s.current_editor() {
                    e.paste()
                }
            }),
            "&Edit/Find\t" => find(),
            "&Edit/Replace\t" => replace(),
            "&View/File browser\t" => {
                let mut item = m.at(m.value()).unwrap();
                let fbr: browser::FileBrowser = app::widget_from_id("fbr").unwrap();
                let mut parent: group::Flex = unsafe { fbr.parent().unwrap().into_widget() };
                if !item.value() {
                    parent.fixed(&fbr, 1);
                    item.clear();
                } else {
                    parent.fixed(&fbr, 180);
                    item.set();
                }
                app::redraw();
            }
            "&View/Terminal\t" => {
                let mut item = m.at(m.value()).unwrap();
                let term: text::TextDisplay = app::widget_from_id("term").unwrap();
                let mut parent: group::Flex = unsafe { term.parent().unwrap().into_widget() };
                if !item.value() {
                    parent.fixed(&term, 1);
                    item.clear();
                } else {
                    parent.fixed(&term, 160);
                    item.set();
                }
                app::redraw();
            }
            "&Help/About\t" => {
                dialog::message_title("About");
                dialog::message_default("A minimal text editor written using fltk-rs!")
            }
            _ => unreachable!(),
        }
    }
}

pub fn tab_close_cb(g: &mut impl GroupExt) {
    if app::callback_reason() == CallbackReason::Closed {
        let ed: text::TextEditor = unsafe { g.child(0).unwrap().into_widget() };
        let edid = ed.as_widget_ptr() as usize;
        let buf = ed.buffer().unwrap();
        let mut parent = g.parent().unwrap();
        parent.remove(g);
        unsafe {
            text::TextBuffer::delete(buf);
        }
        STATE.with(move |s| s.map.remove(&edid));
        parent.set_damage(true);
    }
}

#[cfg(feature = "term")]
pub fn tab_splitter_cb(f: &mut frame::Frame, ev: Event) -> bool {
    let mut parent: group::Flex = unsafe { f.parent().unwrap().into_widget() };
    let term = app::widget_from_id::<group::Group>("term_group").unwrap();
    match ev {
        Event::Push => true,
        Event::Drag => {
            parent.fixed(&term, parent.h() + parent.y() - app::event_y());
            app::redraw();
            true
        }
        Event::Enter => {
            f.window().unwrap().set_cursor(Cursor::NS);
            true
        }
        Event::Leave => {
            f.window().unwrap().set_cursor(Cursor::Arrow);
            true
        }
        _ => false,
    }
}

pub fn fbr_splitter_cb(f: &mut frame::Frame, ev: Event) -> bool {
    let mut parent: group::Flex = unsafe { f.parent().unwrap().into_widget() };
    let fbr: group::Group = app::widget_from_id("fbr_group").unwrap();
    match ev {
        Event::Push => true,
        Event::Drag => {
            parent.fixed(&fbr, app::event_x());
            app::redraw();
            true
        }
        Event::Enter => {
            f.window().unwrap().set_cursor(Cursor::WE);
            true
        }
        Event::Leave => {
            f.window().unwrap().set_cursor(Cursor::Arrow);
            true
        }
        _ => false,
    }
}
