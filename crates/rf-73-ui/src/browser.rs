use crate::client::{Client, PROTOCOL};
use js_sys::{JSON, Object};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, prelude::*};
use web_sys::{
    Document, Element, Event, HtmlElement, HtmlInputElement, HtmlSelectElement, KeyboardEvent,
    MessageEvent, Window,
};

struct App {
    window: Window,
    document: Document,
    origin: String,
    connected: bool,
    client: Client,
}
type Shared = Rc<RefCell<App>>;

const SECTIONS: [&str; 4] = ["hammer", "resonator", "pickup", "output"];

fn select_section(document: &Document, selected: usize) -> Result<(), JsValue> {
    for (index, id) in SECTIONS.iter().enumerate() {
        let tab = document
            .get_element_by_id(&format!("tab-{id}"))
            .expect("section tab");
        tab.set_attribute(
            "aria-selected",
            if index == selected { "true" } else { "false" },
        )?;
        tab.set_attribute("tabindex", if index == selected { "0" } else { "-1" })?;
        let panel = document.get_element_by_id(id).expect("section panel");
        if index == selected {
            panel.remove_attribute("hidden")?;
        } else {
            panel.set_attribute("hidden", "")?;
        }
    }
    Ok(())
}

fn section_events(document: &Document) -> Result<(), JsValue> {
    for (index, id) in SECTIONS.iter().enumerate() {
        let tab = document
            .get_element_by_id(&format!("tab-{id}"))
            .expect("section tab");
        let page = document.clone();
        let click = Closure::<dyn FnMut(Event)>::new(move |_| {
            let _ = select_section(&page, index);
        });
        tab.add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
        click.forget();
        let page = document.clone();
        let key = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            let next = match event.key().as_str() {
                "ArrowRight" => (index + 1) % SECTIONS.len(),
                "ArrowLeft" => (index + SECTIONS.len() - 1) % SECTIONS.len(),
                "Home" => 0,
                "End" => SECTIONS.len() - 1,
                _ => return,
            };
            event.prevent_default();
            let _ = select_section(&page, next);
            if let Some(tab) = page.get_element_by_id(&format!("tab-{}", SECTIONS[next])) {
                let _ = tab.unchecked_into::<HtmlElement>().focus();
            }
        });
        tab.add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref())?;
        key.forget();
    }
    select_section(document, 0)
}

/// Sound-page controls: element id, parameter index, displayed decimals, unit.
const CONTROLS: [(&str, usize, usize, &str); 7] = [
    ("law", 1, 0, ""),
    ("distance", 2, 2, " mm"),
    ("alignment", 3, 2, " mm"),
    ("hardness", 4, 3, ""),
    ("sustain", 5, 3, ""),
    ("bell", 6, 3, ""),
    ("dynamics", 7, 3, ""),
];

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
        for id in CONTROLS.iter().map(|c| c.0).chain(["gain", "gain-number"]) {
            let element = self.element(id);
            if ready {
                let _ = element.remove_attribute("disabled");
            } else {
                let _ = element.set_attribute("disabled", "");
            }
        }
        let focused = self.document.active_element().map(|element| element.id());
        for (id, index, decimals, unit) in CONTROLS {
            let value = self.client.display(index);
            let element = self.element(id);
            if id == "law" {
                element
                    .unchecked_into::<HtmlSelectElement>()
                    .set_value(&format!("{value}"));
                self.text(
                    "law-value",
                    if value == 2.0 {
                        "Register Aperture"
                    } else if value == 1.0 {
                        "Aperture"
                    } else {
                        "Production"
                    },
                );
            } else {
                // A control being dragged keeps its own value until the host answers.
                if focused.as_deref() != Some(id) {
                    element
                        .unchecked_into::<HtmlInputElement>()
                        .set_value_as_number(value);
                }
                self.text(&format!("{id}-value"), &format!("{value:.decimals$}{unit}"));
            }
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
    section_events(&document)?;
    let app = Rc::new(RefCell::new(App {
        window,
        document,
        origin,
        connected: false,
        client: Client::default(),
    }));
    for (id, index, event) in CONTROLS
        .iter()
        .map(|c| (c.0, c.1, if c.0 == "law" { "change" } else { "input" }))
        .chain([("gain", 0, "input"), ("gain-number", 0, "change")])
    {
        parameter_event(&app, id, index, event)?;
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
