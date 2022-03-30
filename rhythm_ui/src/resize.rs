use crate::{get_document_ref, log, show_error_w_val, WebRes};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    window, AddEventListenerOptions, CssStyleDeclaration, Document, Element, Event, HtmlElement,
    MouseEvent,
};

enum OngoingResize {
    None,
    Table {
        col: usize,
        styles: Vec<String>,
        header: HtmlElement,
        style: CssStyleDeclaration,
    },
    Grid {
        col: usize,
        x: bool,
        styles: Vec<u32>,
        gutter: HtmlElement,
        style: CssStyleDeclaration,
    },
}

fn init_resize(evt: MouseEvent) {
    if let Err(e) = move || -> WebRes {
        if let Some(ele) = evt.target() {
            let window = window().expect("no global `window` exists");
            window.add_event_listener_with_callback_and_add_event_listener_options(
                "mouseup",
                unsafe { CB_DONE.as_ref() }.unwrap().unchecked_ref(),
                AddEventListenerOptions::new().once(true),
            )?;
            window.add_event_listener_with_callback(
                "mousemove",
                unsafe { CB_MOVE.as_ref() }.unwrap().unchecked_ref(),
            )?;
            window.add_event_listener_with_callback(
                "selectstart",
                unsafe { NOOP.as_ref() }.unwrap().unchecked_ref(),
            )?;
            window.add_event_listener_with_callback(
                "dragstart",
                unsafe { NOOP.as_ref() }.unwrap().unchecked_ref(),
            )?;

            let ele = ele.dyn_into::<HtmlElement>()?;
            match ele.node_name().as_str() {
                "SPAN" => {
                    //table col resize
                    let ele = ele.parent_element().unwrap();
                    //add class header--being-resized
                    ele.set_class_name("header--being-resized");
                    //convert all width to px
                    //let current_w = ele.offset_width();
                    let re = get_document_ref().query_selector(".ilist")?.unwrap();
                    let col_style = window
                        .get_computed_style(&re)?
                        .unwrap()
                        .get_property_value("grid-template-columns")?;

                    let cols = ele.parent_element().unwrap().children();
                    let mut x = 0;
                    //while x < cols.length()
                    let col = loop {
                        let n = cols.item(x).unwrap();
                        if n == ele {
                            break x;
                        }
                        x += 1;
                    } as usize;

                    let styles = col_style.split(" ").map(|s| s.into()).collect();
                    let style = re.dyn_into::<HtmlElement>()?.style();

                    //set cursor drag
                    get_document_ref()
                        .body()
                        .unwrap()
                        .style()
                        .set_property("cursor", "col-resize")?;
                    //save target col
                    let header = ele.dyn_into::<HtmlElement>()?;
                    unsafe {
                        RESIZED_HEADER = OngoingResize::Table {
                            col,
                            styles,
                            header,
                            style,
                        };
                    }
                }
                "DIV" => {
                    //layout resize
                    let (col, x, attr) = match ele.class_name().as_str() {
                        "gutter-column-1" => (1, true, "grid-template-columns"),
                        "gutter-column-3" => (3, true, "grid-template-columns"),
                        "gutter-row-1" => (1, false, "grid-template-rows"),
                        x => return Err(x.into()),
                    };
                    if x {
                        get_document_ref()
                            .body()
                            .unwrap()
                            .style()
                            .set_property("cursor", "col-resize")?;
                    } else {
                        get_document_ref()
                            .body()
                            .unwrap()
                            .style()
                            .set_property("cursor", "row-resize")?;
                    }

                    let re = get_document_ref().query_selector(".grid")?.unwrap();
                    let col_style = window
                        .get_computed_style(&re)?
                        .unwrap()
                        .get_property_value(attr)?;
                    let styles = col_style
                        .split(" ")
                        .map(|s| s[..s.len() - 2].parse::<f32>().unwrap().round() as u32)
                        .collect();
                    let style = re.dyn_into::<HtmlElement>()?.style();

                    unsafe {
                        RESIZED_HEADER = OngoingResize::Grid {
                            col,
                            x,
                            styles,
                            gutter: ele,
                            style,
                        };
                    }
                }
                x => return Err(x.into()),
            }
        }
        Ok(())
    }() {
        show_error_w_val("error initResize: ", e);
    }
}

fn on_mouse_up(evt: MouseEvent) {
    if let Err(e) = move || -> WebRes {
        let window = window().expect("no global `window` exists");
        window.remove_event_listener_with_callback(
            "mousemove",
            unsafe { CB_MOVE.as_ref() }.unwrap().unchecked_ref(),
        )?;
        window.remove_event_listener_with_callback(
            "selectstart",
            unsafe { NOOP.as_ref() }.unwrap().unchecked_ref(),
        )?;
        window.remove_event_listener_with_callback(
            "dragstart",
            unsafe { NOOP.as_ref() }.unwrap().unchecked_ref(),
        )?;
        //remove cursor drag
        get_document_ref()
            .body()
            .unwrap()
            .style()
            .remove_property("cursor")?;

        let op = unsafe { std::mem::replace(&mut RESIZED_HEADER, OngoingResize::None) };

        match op {
            OngoingResize::Table {
                col,
                styles,
                header,
                style,
            } => {
                //remove class header--being-resized
                header.set_class_name("");
            }
            OngoingResize::Grid {
                col,
                x,
                styles,
                gutter,
                style,
            } => {
                //convert px to fr for nice window resizing
                let mut iter = styles.iter();
                let size_ref = *iter.next().unwrap() as f32;
                let mut s = String::from("1fr ");
                for (x, &space) in iter.enumerate() {
                    if x % 2 == 0 {
                        //gutter
                        s.push_str(&format!("{}px ", space));
                    } else {
                        s.push_str(&format!("{}fr ", space as f32 / size_ref));
                    }
                }
                if x {
                    style.set_property("grid-template-columns", &s)?;
                } else {
                    style.set_property("grid-template-rows", &s)?;
                }
            }
            _ => {}
        }
        Ok(())
    }() {
        show_error_w_val("error resize fin: ", e);
    }
}

fn on_mouse_move(evt: MouseEvent) {
    if let Err(e) = move || -> WebRes {
        match unsafe { &mut RESIZED_HEADER } {
            OngoingResize::Table {
                col,
                styles,
                header,
                style,
            } => {
                // Calculate the desired width
                let w = evt.client_x() - header.offset_left();
                // Enforce our minimum
                if w < 60 {
                    //min size -> do nothing
                    return Ok(());
                }
                styles[*col] = format!("{}px", w);
                //Update the column sizes
                let mut s = String::new();
                for x in styles {
                    s.push_str(x);
                    s.push(' ');
                }
                style.set_property("grid-template-columns", &s)?;
            }
            OngoingResize::Grid {
                col,
                x,
                styles,
                gutter,
                style,
            } => {
                // Calculate the desired width
                let rel_cahnge = if *x {
                    evt.client_x() - gutter.offset_left()
                } else {
                    evt.client_y() - gutter.offset_top()
                };
                // Enforce our minimum
                let tmp = styles[*col - 1] as i32 + rel_cahnge;
                if tmp < 150 {
                    //min size -> do nothing
                    return Ok(());
                }
                styles[*col - 1] = tmp as u32;
                let tmp = styles[*col + 1] as i32 - rel_cahnge;
                if tmp < 150 {
                    //min size -> do nothing
                    return Ok(());
                }
                styles[*col + 1] = tmp as u32;
                //Update the column sizes
                let mut s = String::new();
                for x in styles {
                    s.push_str(&format!("{}px ", *x));
                }
                if *x {
                    style.set_property("grid-template-columns", &s)?;
                } else {
                    style.set_property("grid-template-rows", &s)?;
                }
            }
            _ => {}
        }
        Ok(())
    }() {
        show_error_w_val("error onMouseMove: ", e);
    }
}

fn prevent_action(evt: Event) {
    evt.prevent_default();
}

static mut RESIZED_HEADER: OngoingResize = OngoingResize::None;
static mut CB_MOVE: Option<JsValue> = None;
static mut CB_DONE: Option<JsValue> = None;
static mut NOOP: Option<JsValue> = None;

pub fn setup_resize(document: &Document) -> WebRes {
    //setup callback
    let cb_init = Closure::wrap(Box::new(init_resize) as Box<dyn Fn(MouseEvent)>).into_js_value();
    let cb_move = Closure::wrap(Box::new(on_mouse_move) as Box<dyn Fn(MouseEvent)>);
    let cb_done = Closure::wrap(Box::new(on_mouse_up) as Box<dyn Fn(MouseEvent)>);
    let noop = Closure::wrap(Box::new(prevent_action) as Box<dyn Fn(Event)>);
    let ucref = cb_init.unchecked_ref();
    unsafe {
        CB_MOVE = Some(cb_move.into_js_value());
        CB_DONE = Some(cb_done.into_js_value());
        NOOP = Some(noop.into_js_value());
    };

    //add callback to all headers and all gutters
    let nodes = document.query_selector_all(".ilist thead th span, .grid div:nth-child(even)")?;
    let mut x = 0;
    while x < nodes.length() {
        let n = nodes.get(x).unwrap();
        n.add_event_listener_with_callback("mousedown", ucref)?;
        x += 1;
    }
    Ok(())
}
