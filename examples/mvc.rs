use std::cell::RefCell;
use std::rc::Rc;

pub struct Model {
    pub text: String,
    listeners: Vec<View>,
}

impl Model {
    fn new() -> Model {
        Model {
            text: String::new(),
            listeners: Vec::new(),
        }
    }

    fn set_text(&mut self, new_text: &str) {
        self.text = new_text.to_owned();
        self.notify_listeners();
    }

    fn get_text(&self) -> String {
        self.text.to_owned()
    }

    fn add_listener(&mut self, listener: View) {
        self.listeners.push(listener);
    }

    fn notify_listeners(&self) {
        for listener in &self.listeners {
            listener.update(&self);
        }
    }
}

#[derive(Clone)]
pub struct View {
    upper: StatusBar,
    lower: StatusBar,
}

impl View {
    pub fn new(upper: StatusBar, lower: StatusBar) -> View {
        View {
            upper,
            lower,
        }
    }

    fn draw(&self) {
        self.upper.draw();
        self.lower.draw();
    }

    fn update(&self, model: &Model) {
        self.lower.set_text(&model.get_text());
        self.draw();
    }

    fn scroll(&self) {
        println!("Scroll...");
    }
}

#[derive(Clone)]
pub struct StatusBar {
    data: Rc<RefCell<StatusBarData>>,
}

#[derive(Clone)]
struct StatusBarData {
    text: String,
}

impl StatusBar {
    fn new() -> StatusBar {
        StatusBar {
            data: Rc::new(
                RefCell::new(
                    StatusBarData {
                        text: String::new(),
                    }
                )
            )
        }
    }

    fn draw(&self) {
        println!("-{}", self.data.borrow().text);
    }

    fn set_text(&self, new_text: &str) {
        self.data.borrow_mut().text = new_text.to_owned();
    }

    fn get_text(&self) -> String {
        self.data.borrow().text.to_owned()
    }
}

pub struct Controller {
    model: Model,
    view: View,
}

impl Controller {
    pub fn new(model: Model, view: View) -> Controller {
        Controller {
            model,
            view,
        }
    }

    pub fn change_text(&mut self, new_text: &str) {
        self.model.set_text(new_text);
    }

    pub fn scroll(&mut self) {
        self.view.scroll();
    }
}

fn main() {
    let mut controller = {
        let mut model = Model::new();

        let view1 = StatusBar::new();
        let view2 = StatusBar::new();
        let view = View::new(view1, view2);

        model.add_listener(view.clone());

        Controller::new(model, view)
    };

    controller.change_text("Hello,");
    controller.change_text("World!");

    controller.scroll();
}