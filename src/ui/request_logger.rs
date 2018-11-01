use gtk::prelude::*;
use gtk::{self, Orientation, ScrolledWindow};
use relm::{self, Relm, Update, Widget};
use sourceview::{self, prelude::*, LanguageManager, StyleSchemeManager, View as SourceView};

use super::super::helpers::http::{Http, Msg as HttpMsg};

#[derive(Msg)]
pub enum Msg {
    ExecutingRequest(String),
    RequestExecuted(String),
}

pub struct RequestLogger {
    hbox: gtk::Box,
    logger_view: SourceView,
    relm: Relm<RequestLogger>,
}

impl Update for RequestLogger {
    type Model = ();
    type ModelParam = ();
    type Msg = Msg;

    fn model(_: &Relm<Self>, _: ()) -> () {
        ()
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::ExecutingRequest(request) => {
                let buffer = self.logger_view.get_buffer().unwrap();

                let start_iter = buffer.get_start_iter();
                let end_iter = buffer.get_end_iter();
                let mut current = match buffer.get_text(&start_iter, &end_iter, true) {
                    Some(data) => data,
                    None => "".to_string(),
                };
                current.push_str(">>> New Request\n");
                current.push_str(request.as_str());
                current.push_str("\n\n");

                let http = relm::execute::<Http>(request);
                connect_stream!(
                    http@HttpMsg::ReadDone(ref response), self.relm.stream(), Msg::RequestExecuted(response.clone()));
                buffer.set_text(current.as_str());
            }

            Msg::RequestExecuted(response) => {
                let buffer = self.logger_view.get_buffer().unwrap();
                let start_iter = buffer.get_start_iter();
                let end_iter = buffer.get_end_iter();
                let mut current = match buffer.get_text(&start_iter, &end_iter, true) {
                    Some(data) => data,
                    None => "".to_string(),
                };
                current.push_str("<<< Response ***\n");
                current.push_str(response.as_str());
                current.push_str("\n\n");
                buffer.set_text(current.as_str());
            }
        }
    }
}

impl Widget for RequestLogger {
    type Root = gtk::Box;

    fn root(&self) -> Self::Root {
        self.hbox.clone()
    }

    fn view(relm: &Relm<Self>, _model: ()) -> Self {
        info!("Creating RequestLogger widget");
        let hbox = gtk::Box::new(Orientation::Horizontal, 0);

        let langmngr = LanguageManager::get_default().unwrap();
        let lang = langmngr.get_language("rustaman-response").unwrap();

        let stylemngr = StyleSchemeManager::get_default().unwrap();
        let style = stylemngr.get_scheme("solarized-dark").unwrap();

        let buffer = sourceview::Buffer::new_with_language(&lang);
        buffer.set_style_scheme(&style);

        let logger_view = SourceView::new_with_buffer(&buffer);
        logger_view.set_hexpand(true);
        logger_view.set_vexpand(true);
        logger_view.set_editable(false);

        let scrollview = ScrolledWindow::new(None, None);
        scrollview.add(&logger_view);

        hbox.set_margin_top(5);
        hbox.set_margin_bottom(5);
        hbox.pack_start(&scrollview, true, true, 5);

        hbox.show_all();
        RequestLogger {
            hbox,
            logger_view,
            relm: relm.clone(),
        }
    }
}
