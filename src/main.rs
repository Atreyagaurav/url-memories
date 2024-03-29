use dirs::data_local_dir;
use gtk::{glib, prelude::*};
use gtk::{Application, StringList, StringObject};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use string_template_plus::{Render, RenderOptions, Template};

fn memory_path() -> PathBuf {
    data_local_dir().unwrap().join(".url-memories")
}

#[derive(Debug)]
struct AnimeEntry {
    category: String,
    title: String,
    url_template: Template,
    watched: String,
    timestamp: u64,
    note: String,
}

impl FromStr for AnimeEntry {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cols: Vec<&str> = s.splitn(6, ',').collect();
        Ok(AnimeEntry {
            category: cols[0].to_string(),
            title: cols[1].to_string(),
            url_template: Template::parse_template(cols[2])?,
            watched: cols[3].to_string(),
            timestamp: cols[4].parse()?,
            note: cols.get(5).unwrap_or(&"").to_string(),
        })
    }
}

impl std::fmt::Display for AnimeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{},{},{},{},{}",
            self.category,
            self.title,
            self.url_template.original(),
            self.watched,
            self.timestamp,
            self.note,
        )
    }
}

impl AnimeEntry {
    fn new(title: String, url: &str, eps: String) -> Self {
        Self {
            category: "NA".to_string(),
            title,
            url_template: Template::parse_template(&url).unwrap(),
            watched: eps,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs(),
            note: "".to_string(),
        }
    }

    fn get_url(&self, eps: usize) -> String {
        let mut op = RenderOptions::default();
        op.variables.insert("episode".to_string(), eps.to_string());
        self.url_template.render(&op).unwrap()
    }

    fn get_note(&self) -> &str {
        &self.note
    }

    fn update(&mut self, eps: &str, note: &str) {
        self.watched = eps.to_string();
        self.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        self.note = note.to_string();
    }
}

trait FileIO {
    fn load(&mut self);
    fn save(&self);
}

impl FileIO for HashMap<String, AnimeEntry> {
    fn load(&mut self) {
        if let Ok(mem_file) = File::open(&memory_path()) {
            let mut lines = BufReader::new(mem_file).lines();
            lines.next(); // discard header line;
            let memories: HashMap<String, AnimeEntry> = lines
                .filter_map(|line| AnimeEntry::from_str(&line.unwrap()).ok())
                .map(|a| (a.title.clone(), a))
                .collect();
            self.extend(memories);
        }
    }

    fn save(&self) {
        let mem_file = File::create(&memory_path()).unwrap();
        let mut writer = BufWriter::new(mem_file);
        writeln!(writer, "category,title,url_template,watched,timestamp,note").unwrap();
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

    // watch tab
    load_ui!(dd_memory, gtk::DropDown);
    load_ui!(txt_eps, gtk::Entry);
    load_ui!(txt_note, gtk::Entry);
    load_ui!(cb_open, gtk::CheckButton);
    load_ui!(btn_link, gtk::Button);
    load_ui!(btn_next, gtk::Button);
    load_ui!(btn_save, gtk::Button);
    // add new tab
    load_ui!(ent_title, gtk::Entry);
    load_ui!(ent_url, gtk::Entry);
    load_ui!(ent_eps, gtk::Entry);
    load_ui!(btn_save_new, gtk::Button);

    btn_link.connect_clicked(
	glib::clone!(@weak txt_eps, @weak dd_memory, @weak memories => move |_| {
	let eps: usize = number_range::NumberRangeOptions::default().with_range_sep('-').parse(&txt_eps.text()).unwrap().last().unwrap();
	    let memory = dd_memory.selected_item().unwrap().downcast::<StringObject>().unwrap().string().to_string();
	    open::that(memories.borrow().get(&memory).unwrap().get_url(eps)).unwrap();
        }));

    btn_next.connect_clicked(
	glib::clone!(@weak txt_eps, @weak dd_memory, @weak cb_open, @weak txt_note, @weak memories => move |_| {
	let mut eps: Vec<usize> = number_range::NumberRangeOptions::default().with_range_sep('-').parse(&txt_eps.text().to_string()).unwrap().collect();
	    let memory = dd_memory.selected_item().unwrap().downcast::<StringObject>().unwrap().string().to_string();
	    let next = eps[eps.len() - 1] + 1;
	    let mems = memories.borrow();
	    let mem = mems.get(&memory).unwrap();
	    let url = mem.get_url(next);
	    if cb_open.is_active(){
		open::that(url).unwrap();
	    }
	    eps.push(next);
	    txt_eps.set_text(&number_range::NumberRangeOptions::default().with_range_sep('-').parse("").unwrap().from_vec(eps, Some(1)).to_string());
        }));

    btn_save.connect_clicked(
	glib::clone!(@weak txt_eps, @weak dd_memory, @weak txt_note, @weak memories  => move |_| {
	    let memory = dd_memory.selected_item().unwrap().downcast::<StringObject>().unwrap().string().to_string();
	    let eps = txt_eps.text();
	    let note = txt_note.text();
	    memories.borrow_mut().get_mut(&memory).unwrap().update(&eps, &note);
	    memories.borrow().save();
        }));

    btn_save_new.connect_clicked(
        glib::clone!(@weak ent_eps, @weak dd_memory, @weak ent_title, @weak ent_url, @weak memories  => move |_| {
            let title = ent_title.text();
            let url = ent_url.text();
            let eps = ent_eps.text();
            if !title.is_empty() && !url.is_empty() && !eps.is_empty() {
		memories.borrow_mut().insert(title.to_string(), AnimeEntry::new(title.to_string(), &url, eps.to_string()));
		memories.borrow().save();
		ent_title.set_text("");
		ent_url.set_text("");
		ent_eps.set_text("");
		let memlst: StringList = memories.borrow().keys().map(|a| a.to_string()).collect();
		dd_memory.set_model(Some(&memlst));
            }
        }),
    );

    dd_memory.connect_selected_item_notify(
        glib::clone!(@weak txt_eps, @weak txt_note, @strong memories  => move |dd_memory| {
            if let Some(memory) = dd_memory.selected_item().map( |i| i.downcast::<StringObject>().unwrap()){
		let mems = memories.borrow();
		let mem = mems.get(&memory.string().to_string()).unwrap();
		txt_eps.set_text(&mem.watched);
		txt_note.set_text(&mem.get_note());
            }}));

    memories.borrow_mut().load();
    let memlst: StringList = {
        let mem = memories.borrow();
        let mut sorted_al: Vec<&AnimeEntry> = mem.values().collect();
        sorted_al.sort_by(|a, b| b.timestamp.partial_cmp(&a.timestamp).unwrap());
        sorted_al.iter().map(|a| a.title.to_string()).collect()
    };
    dd_memory.set_model(Some(&memlst));

    window.present();
}
