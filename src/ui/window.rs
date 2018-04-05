use gdk;
use gdk::enums::key;
use gtk::{self, Orientation, WindowPosition, WindowType};
use gtk::prelude::*;
use glib::translate::ToGlib;
use relm::{Component, ContainerWidget, Relm, Update, Widget};

use super::super::models::{Environment, Request, RequestRunner, Template, Workspace};
use super::menu::{Menu, Msg as MenuMsg};
use super::request_editor::{Msg as EditorMsg, RequestEditor};
use super::response::{Msg as ResponseMsg, Response};
use super::environ_editor::{EnvironEditor, Msg as EnvironMsg};
use super::helpbox::{HelpBox, Msg as HelpBoxMsg};

#[derive(Msg)]
pub enum Msg {
    CreatingRequest,
    TogglingRequest(usize, bool),
    Deleting(usize),
    Renaming(usize, String),
    RequestTemplateChanged(usize, Template),
    ExecutingRequestTemplate(Template),
    ExecutingCurrentRequestTemplate,
    TemplateCompiled(String),
    TemplateCompilationFailed(String),
    SavingEnvironment(usize, Environment),
    CreatingEnvironment(String),
    Quitting,
    PressingKey(gdk::EventKey),
}

pub struct Model {
    workspace: Workspace,
    current: usize,
}

impl Model {
    pub fn name(&self) -> &str {
        self.workspace.name()
    }
    pub fn requests(&self) -> &[Request] {
        self.workspace.requests()
    }
    pub fn current_request(&self) -> Option<&Request> {
        self.workspace.request(self.current)
    }
    pub fn create_request(&mut self) -> &Request {
        self.workspace.create_request()
    }
    pub fn environments(&self) -> &[Environment] {
        self.workspace.environments()
    }
    pub fn create_environment(&mut self, name: &str) -> &Environment {
        self.workspace.create_environment(name)
    }
}

pub struct Window {
    model: Model,
    menu: Component<Menu>,
    window: gtk::Window,
    relm: Relm<Window>,
    request_editor: Component<RequestEditor>,
    env_editor: Component<EnvironEditor>,
    help_box: Component<HelpBox>,
    response: Component<Response>,
    runner: RequestRunner,
}

impl Update for Window {
    type Model = Model;
    type ModelParam = Workspace;
    type Msg = Msg;

    fn model(_: &Relm<Self>, workspace: Workspace) -> Model {
        Model {
            workspace: workspace,
            current: 0,
        }
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::CreatingRequest => {
                let request = self.model.create_request();
                self.menu
                    .stream()
                    .emit(MenuMsg::CreatingRequest(request.clone()))
            }
            Msg::TogglingRequest(id, active) => {
                if self.model.current > 0 {
                    self.request_editor
                        .stream()
                        .emit(EditorMsg::RequestingSave(self.model.current));
                }
                if active {
                    self.model.current = id;
                    let req = self.model.current_request().unwrap(); // XXX
                    self.help_box.stream().emit(HelpBoxMsg::Hiding);
                    self.request_editor
                        .stream()
                        .emit(EditorMsg::TemplateChanged(req.template().to_owned()));
                } else if self.model.current == id {
                    self.request_editor.stream().emit(EditorMsg::Hiding);
                    self.help_box.stream().emit(HelpBoxMsg::Showing);
                    self.model.current = 0;
                }
            }
            Msg::Renaming(id, name) => {
                self.model.workspace.set_request_name(id, name.as_str());
            }
            Msg::Deleting(id) => {
                info!("Deleting template {}", id);
                self.model.workspace.delete_request(id);

                if self.model.current == id {
                    self.request_editor.stream().emit(EditorMsg::Hiding);
                    self.help_box.stream().emit(HelpBoxMsg::Showing);
                    self.model.current = 0;
                }

                self.menu.stream().emit(MenuMsg::Deleted(id));
            }
            Msg::RequestTemplateChanged(id, template) => {
                info!("Save Template Changes {} {}", id, template);
                self.model
                    .workspace
                    .set_request_template(id, template.as_str());
            }
            Msg::ExecutingRequestTemplate(template) => {
                self.relm.stream().emit(Msg::RequestTemplateChanged(
                    self.model.current,
                    template.clone(),
                ));

                self.env_editor
                    .stream()
                    .emit(EnvironMsg::CompilingTemplate(template));
            }
            Msg::ExecutingCurrentRequestTemplate => {
                self.request_editor
                    .stream()
                    .emit(EditorMsg::ExecutingCurrent);
            }
            Msg::TemplateCompiled(template) => {
                let resp = self.runner.run_request(template.as_str());
                self.response
                    .stream()
                    .emit(ResponseMsg::RequestExecuted(resp));
            }
            Msg::TemplateCompilationFailed(msg) => {
                self.response
                    .stream()
                    .emit(ResponseMsg::RequestExecuted(msg));
            }
            Msg::CreatingEnvironment(name) => {
                info!("Creating environment {}", name);
                let env = self.model.create_environment(name.as_str());
                self.env_editor
                    .stream()
                    .emit(EnvironMsg::EnvironmentCreated(env.clone()))
            }
            Msg::SavingEnvironment(id, env) => {
                info!("Save environment {} {:?}", id, env);
                self.model.workspace.set_environ(id, &env);
            }
            Msg::Quitting => gtk::main_quit(),
            Msg::PressingKey(key) => {
                let keyval = key.get_keyval();
                let keystate = key.get_state();

                if keystate.intersects(gdk::ModifierType::CONTROL_MASK) {
                    match keyval {
                        key::w => self.relm.stream().emit(Msg::Quitting),
                        key::n => self.relm.stream().emit(Msg::CreatingRequest),
                        key::p => self.menu.stream().emit(MenuMsg::RequestingFilteringMenu),
                        key::Return => self.relm
                            .stream()
                            .emit(Msg::ExecutingCurrentRequestTemplate),
                        _ => {}
                    }
                } else {
                    match keyval {
                        key::F2 => {
                            if self.model.current > 0 {
                                self.menu
                                    .stream()
                                    .emit(MenuMsg::RenamingRequest(self.model.current))
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

impl Widget for Window {
    type Root = gtk::Window;

    fn root(&self) -> Self::Root {
        self.window.clone()
    }

    fn view(relm: &Relm<Self>, model: Model) -> Self {
        let window = gtk::Window::new(WindowType::Toplevel);

        window.set_title(model.name());
        window.set_border_width(10);
        window.set_position(WindowPosition::Center);
        window.set_default_size(1280, 1024);

        connect!(
            relm,
            window,
            connect_delete_event(_, _),
            return (Some(Msg::Quitting), Inhibit(false))
        );

        connect!(
            relm,
            window,
            connect_key_press_event(_, key),
            return (Msg::PressingKey(key.clone()), Inhibit(false))
        );

        let settings = gtk::Settings::get_default().unwrap();
        let use_dark = true;
        settings.set_long_property(
            "gtk-application-prefer-dark-theme",
            use_dark.to_glib() as _,
            "",
        );

        let hbox = gtk::Box::new(Orientation::Horizontal, 0);
        hbox.set_hexpand(true);
        hbox.set_vexpand(true);
        let requests = model.requests().to_vec();
        let menu = hbox.add_widget::<Menu, _>(relm, requests);
        window.set_hexpand(true);
        window.set_vexpand(true);

        connect!(
            menu@MenuMsg::NewRequest,
            relm,
            Msg::CreatingRequest
        );

        connect!(
            menu@MenuMsg::TogglingRequest(id, active),
            relm,
            Msg::TogglingRequest(id, active)
        );

        connect!(
            menu@MenuMsg::Renaming(id, ref name),
            relm,
            Msg::Renaming(id, name.to_owned())
        );

        connect!(
            menu@MenuMsg::Deleting(id),
            relm,
            Msg::Deleting(id)
        );

        hbox.show_all();

        let main_box = gtk::Box::new(Orientation::Horizontal, 0);
        let editor_box = gtk::Box::new(Orientation::Vertical, 0);
        let editor = editor_box.add_widget::<RequestEditor, _>(relm, ());
        let help_box = editor_box.add_widget::<HelpBox, _>(relm, ());
        connect!(
            editor@EditorMsg::Saving(id, ref template),
            relm,
            Msg::RequestTemplateChanged(id, template.to_owned())
        );
        connect!(
            editor@EditorMsg::Executing(ref template),
            relm,
            Msg::ExecutingRequestTemplate(template.to_owned())
        );
        let envs = model.environments().to_vec();
        let env_editor = editor_box.add_widget::<EnvironEditor, _>(relm, envs);

        connect!(
            env_editor@EnvironMsg::TemplateCompiled(ref result),
            relm,
            Msg::TemplateCompiled(result.to_owned())
        );

        connect!(
            env_editor@EnvironMsg::TemplateCompilationFailed(ref err),
            relm,
            Msg::TemplateCompilationFailed(err.to_owned())
        );

        connect!(
            env_editor@EnvironMsg::CreatingEnvironment(ref name),
            relm,
            Msg::CreatingEnvironment(name.to_owned())
        );

        connect!(
            env_editor@EnvironMsg::Saving(id, ref env),
            relm,
            Msg::SavingEnvironment(id, env.clone())
        );

        if model.environments().len() == 0 {
            relm.stream()
                .emit(Msg::CreatingEnvironment("Dev".to_owned()));
        }

        main_box.add(&editor_box);

        let response = main_box.add_widget::<Response, _>(relm, ());
        editor_box.show();
        main_box.show();
        hbox.add(&main_box);
        window.add(&hbox);
        window.show();
        Window {
            model: model,
            menu: menu,
            window: window,
            request_editor: editor,
            env_editor: env_editor,
            help_box: help_box,
            response: response,
            relm: relm.clone(),
            runner: RequestRunner::new(),
        }
    }
}
