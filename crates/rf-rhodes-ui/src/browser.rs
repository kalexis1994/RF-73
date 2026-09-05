use crate::client::{Client, PROTOCOL};
use js_sys::{JSON, Object};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{
    Document, Element, Event, HtmlInputElement, HtmlSelectElement, MessageEvent, Window,
};

struct App {
    window: Window,
    document: Document,
    origin: String,
    connected: bool,
    client: Client,
}
type Shared = Rc<RefCell<App>>;

impl App {
    fn element(&self, id: &str) -> Element {
        self.document
            .get_element_by_id(id)
            .expect("static UI element")
    }
    fn text(&self, id: &str, text: &str) {
        self.element(id).set_text_content(Some(text));
    }
    fn render(&self) {
        let ready = self.connected && self.client.loaded;
        for id in [
            "pickup-a",
            "pickup-b",
            "listen-a",
            "listen-b",
            "gain",
            "gain-number",
        ] {
            let element = self.element(id);
            if ready {
                let _ = element.remove_attribute("disabled");
            } else {
                let _ = element.set_attribute("disabled", "");
            }
        }
        let focused = self.document.active_element().map(|element| element.id());
        for (id, index) in [("pickup-a", 1), ("pickup-b", 2)] {
            self.element(id)
                .unchecked_into::<HtmlSelectElement>()
                .set_value(&format!("{}", self.client.display(index)));
        }
        let gain = self.client.display(0);
        for id in ["gain", "gain-number"] {
            // Only the numeric text field can contain an uncommitted edit.
            // Focus alone must not freeze sliders or selectors during MIDI automation.
            if id != "gain-number" || focused.as_deref() != Some(id) {
                self.element(id)
                    .unchecked_into::<HtmlInputElement>()
                    .set_value_as_number(gain);
            }
        }
        let side = self.client.display(3) == 1.0;
        for (id, selected) in [("listen-a", !side), ("listen-b", side)] {
            let _ = self
                .element(id)
                .set_attribute("aria-pressed", if selected { "true" } else { "false" });
        }
        let names = ["Current", "Close Original", "Close Point Pole"];
        let selected = self.client.display(if side { 2 } else { 1 }) as usize;
        self.text(
            "now-playing",
            &format!("{} / {}", if side { "B" } else { "A" }, names[selected]),
        );
        self.text(
            "gain-db",
            &if gain > 0.0 {
                format!("{:.1} dB", 20.0 * gain.log10())
            } else {
                "Muted".into()
            },
        );
        self.text("status", &self.client.status);
        let _ = self
            .element("status")
            .set_attribute("data-ready", if ready { "true" } else { "false" });
    }

    fn send(&self, message: &Value) -> Result<(), JsValue> {
        let parent = self
            .window
            .parent()?
            .ok_or_else(|| JsValue::from_str("missing host"))?;
        let data = JSON::parse(&message.to_string())?;
        parent.post_message(&data, &self.origin)
    }

    fn pump(&mut self, poll: bool) {
        if !self.connected {
            return;
        }
        let now = self
            .window
            .performance()
            .expect("browser monotonic clock")
            .now();
        if let Some(request) = self.client.next(now, poll)
            && self.send(&request).is_err()
        {
            self.client.loaded = false;
            self.client.status = "Could not contact RackForge. Reconnecting...".into();
        }
        self.render();
    }
}

fn parameter_event(app: &Shared, id: &str, index: usize, event_name: &str) -> Result<(), JsValue> {
    let element = app.borrow().element(id);
    let control = element.clone();
    let app = app.clone();
    let callback = Closure::<dyn FnMut(Event)>::new(move |_| {
        let value = if let Some(input) = control.dyn_ref::<HtmlInputElement>() {
            input.value_as_number()
        } else {
            control
                .unchecked_ref::<HtmlSelectElement>()
                .value()
                .parse()
                .unwrap_or(f64::NAN)
        };
        let mut app = app.borrow_mut();
        app.client.queue(index, value);
        app.pump(false);
    });
    element.add_event_listener_with_callback(event_name, callback.as_ref().unchecked_ref())?;
    callback.forget();
    Ok(())
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("missing window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("missing document"))?;
    let origin = window.location().origin()?;
    let app = Rc::new(RefCell::new(App {
        window,
        document,
        origin,
        connected: false,
        client: Client::default(),
    }));
    for (id, index, event) in [
        ("pickup-a", 1, "change"),
        ("pickup-b", 2, "change"),
        ("gain", 0, "input"),
        ("gain-number", 0, "change"),
    ] {
        parameter_event(&app, id, index, event)?;
    }
    for (id, value) in [("listen-a", 0.0), ("listen-b", 1.0)] {
        let element = app.borrow().element(id);
        let app = app.clone();
        let callback = Closure::<dyn FnMut(Event)>::new(move |_| {
            let mut app = app.borrow_mut();
            app.client.queue(3, value);
            app.pump(false);
        });
        element.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    let messages = app.clone();
    let callback = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let mut app = messages.borrow_mut();
        let parent = app
            .window
            .parent()
            .ok()
            .flatten()
            .zip(event.source())
            .is_some_and(|(parent, source)| Object::is(parent.as_ref(), source.as_ref()));
        if !parent || event.origin() != app.origin {
            return;
        }
        let Some(text) = JSON::stringify(&event.data())
            .ok()
            .and_then(|text| text.as_string())
        else {
            return;
        };
        if text.len() > 262144 {
            return;
        }
        let Ok(message) = serde_json::from_str::<Value>(&text) else {
            return;
        };
        if message["protocol"] != PROTOCOL {
            return;
        }
        match message["kind"].as_str() {
            Some("context") if message["instance"]["plugin_id"] == "org.rackforge.rhodes" => {
                let first = !app.connected;
                app.connected = true;
                if let Some(lighting @ ("day" | "stage")) = message["host"]["lighting"].as_str() {
                    let _ = app
                        .document
                        .document_element()
                        .expect("HTML root")
                        .set_attribute("data-lighting", lighting);
                }
                app.pump(first);
            }
            Some("response") => {
                app.client.response(&message);
                app.pump(false);
            }
            _ => {}
        }
    });
    app.borrow()
        .window
        .add_event_listener_with_callback("message", callback.as_ref().unchecked_ref())?;
    callback.forget();
    let timer = app.clone();
    let callback = Closure::<dyn FnMut()>::new(move || {
        timer.borrow_mut().pump(true);
    });
    app.borrow()
        .window
        .set_interval_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            500,
        )?;
    callback.forget();
    app.borrow().render();
    app.borrow()
        .send(&json!({"protocol": PROTOCOL, "kind": "ready"}))
}
