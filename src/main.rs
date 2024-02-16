use gtk::{glib, prelude::*};
use gtk::{Application, StringList, StringObject};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::rc::Rc;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use string_template_plus::{Render, RenderOptions, Template};

#[derive(Debug)]
struct AnimeEntry {
    category: String,
    title: String,
    url_template: Template,
    watched: String,
    timestamp: u64,
}

impl FromStr for AnimeEntry {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cols: Vec<&str> = s.split(',').collect();
        Ok(AnimeEntry {
            category: cols[0].to_string(),
            title: cols[1].to_string(),
            url_template: Template::parse_template(cols[2])?,
            watched: cols[3].to_string(),
            timestamp: cols[4].parse()?,
        })
    }
}

impl std::fmt::Display for AnimeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{},{},{},{}",
            self.category,
            self.title,
            self.url_template.original(),
            self.watched,
            self.timestamp
        )
    }
}

impl AnimeEntry {
    fn get_url(&self, eps: u8) -> String {
        let mut op = RenderOptions::default();
        op.variables.insert("episode".to_string(), eps.to_string());
        self.url_template.render(&op).unwrap()
    }

    fn update(&mut self, eps: &str) {
        self.watched = eps.to_string();
        self.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
    }
}

trait FileIO {
    fn load(&mut self);
    fn save(&self);
}

impl FileIO for HashMap<String, AnimeEntry> {
    fn load(&mut self) {
        let mem_file = File::open(SAVE_FILEPATH).unwrap();
        let mut lines = BufReader::new(mem_file).lines();
        lines.next(); // discard header line;
        let memories: HashMap<String, AnimeEntry> = lines
            .filter_map(|line| AnimeEntry::from_str(&line.unwrap()).ok())
            .map(|a| (a.title.clone(), a))
            .collect();
        self.extend(memories);
    }

    fn save(&self) {
        let mem_file = File::create(SAVE_FILEPATH).unwrap();
        let mut writer = BufWriter::new(mem_file);
        writeln!(writer, "category,title,url_template,watched,timestamp").unwrap();
        for ent in self.values() {
            writeln!(writer, "{}", ent).unwrap();
        }
    }
}

pub const APP_ID: &str = "org.anek.URLMemories";

fn main() -> anyhow::Result<()> {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run();
    Ok(())
}

pub const SAVE_FILEPATH: &str = "/home/gaurav/.url-memories";

pub fn build_ui(application: &gtk::Application) {
    let ui_src = include_str!("../resources/window.ui");
    let builder = gtk::Builder::from_string(ui_src);

    macro_rules! load_ui {
        ($l:ident, $t:ty) => {
            let $l = builder
                .object::<$t>(stringify!($l))
                .expect(concat!("couldn't get: ", stringify!($l)));
        };
    }

    let memories: Rc<RefCell<HashMap<String, AnimeEntry>>> = Rc::new(RefCell::new(HashMap::new()));

    let window = builder
        .object::<gtk::ApplicationWindow>("window")
        .expect("Couldn't get window");
    window.set_application(Some(application));

    load_ui!(dd_memory, gtk::DropDown);
    load_ui!(txt_eps, gtk::Entry);
    load_ui!(cb_open, gtk::CheckButton);
    load_ui!(btn_link, gtk::Button);
    load_ui!(btn_next, gtk::Button);
    load_ui!(btn_save, gtk::Button);
    btn_link.connect_clicked(
	glib::clone!(@weak txt_eps, @weak dd_memory, @weak memories => move |_| {
	let eps: u8 = number_range::NumberRangeOptions::default().with_range_sep('-').parse(&txt_eps.text()).unwrap().last().unwrap();
	    let memory = dd_memory.selected_item().unwrap().downcast::<StringObject>().unwrap().string().to_string();
	    open::that(memories.borrow().get(&memory).unwrap().get_url(eps)).unwrap();
        }));

    btn_next.connect_clicked(
	glib::clone!(@weak txt_eps, @weak dd_memory, @weak cb_open, @weak memories => move |_| {
	let mut eps: Vec<u8> = number_range::NumberRangeOptions::default().with_range_sep('-').parse(&txt_eps.text().to_string()).unwrap().collect();
	    let memory = dd_memory.selected_item().unwrap().downcast::<StringObject>().unwrap().string().to_string();
	    let next = eps[eps.len() - 1] + 1;
	    if cb_open.is_active(){
		open::that(memories.borrow().get(&memory).unwrap().get_url(next)).unwrap();
	    }
	    eps.push(next);
	    txt_eps.set_text(&number_range::NumberRangeOptions::default().with_range_sep('-').parse("").unwrap().from_vec(eps, Some(1)).to_string());
        }));

    btn_save.connect_clicked(
	glib::clone!(@weak txt_eps, @weak dd_memory, @weak memories  => move |_| {
	    let memory = dd_memory.selected_item().unwrap().downcast::<StringObject>().unwrap().string().to_string();
	    let eps = txt_eps.text();
	    memories.borrow_mut().get_mut(&memory).unwrap().update(&eps);
	    memories.borrow().save();
        }));

    dd_memory.connect_selected_item_notify(
        glib::clone!(@weak txt_eps, @strong memories  => move |dd_memory| {
            if let Some(memory) = dd_memory.selected_item().map( |i| i.downcast::<StringObject>().unwrap()){
		let eps = memories.borrow().get(&memory.string().to_string()).unwrap().watched.clone();
		txt_eps.set_text(&eps);
            }}));

    memories.borrow_mut().load();
    let memlst: StringList = memories.borrow().keys().map(|a| a.to_string()).collect();
    dd_memory.set_model(Some(&memlst));

    window.present();
}
