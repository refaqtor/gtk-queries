use gtk::*;
use gtk::prelude::*;
// use gio::prelude::*;
use gtk_plots::mapping_menu::*;
//use crate::PlotSidebar;
use std::rc::Rc;
use std::cell::RefCell;
use tables::{environment_source::EnvironmentSource, TableEnvironment};
use gtkplotview::plot_view::{PlotView, UpdateContent};
use std::path::PathBuf;
use std::fs::File;
use std::io::Read;
use gio::FileExt;
use gtk_plots::design_menu::*;
use gtk_plots::scale_menu::*;
use std::collections::HashMap;
use crate::utils;

/// PlotsSidebar holds the information of the used mappings
#[derive(Clone)]
pub struct PlotSidebar {
    pub design_menu : DesignMenu,
    pub scale_menus : (ScaleMenu, ScaleMenu),
    pub mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
    pub notebook : Notebook,
    pub layout_stack : Stack
}

impl PlotSidebar {

    pub fn new(
        pl_view : Rc<RefCell<PlotView>>,
        table_env : Rc<RefCell<TableEnvironment>>
    ) -> Self {
        let builder = Builder::new_from_file(utils::glade_path("gtk-plots-stack.glade").unwrap());
        let mapping_menus : Vec<MappingMenu> = Vec::new();
        let mapping_menus = Rc::new(RefCell::new(mapping_menus));
        let design_menu = build_design_menu(&builder, pl_view.clone());
        let plot_notebook : Notebook =
        builder.get_object("plot_notebook").unwrap();
        let scale_menus = build_scale_menus(&builder, pl_view.clone());
        let layout_stack : Stack = builder.get_object("layout_stack").unwrap();
        let sidebar = PlotSidebar {
            design_menu,
            scale_menus,
            mapping_menus,
            notebook : plot_notebook.clone(),
            layout_stack : layout_stack.clone()
        };
        let _layout_menu = LayoutMenu::new_from_builder(
            &builder,
            pl_view.clone(),
            table_env.clone(),
            sidebar.clone()
        );
        sidebar
    }

    /*pub fn used_names_and_types(&self) -> (Vec<String>, Vec<String>) {
        let mut names = Vec::new();
        let mut types = Vec::new();
        println!("Ref Count: {}", Rc::strong_count(&self.mapping_menus));
        if let Ok(_) = self.mapping_menus.try_borrow() {
            println!("Could borrow at used_names_and_types");
        } else {
            println!("Could not borrow at used_names_and_types");
        }

        match self.mapping_menus.try_borrow() {
            Ok(m_menus) =>  {
                for m in m_menus.iter() {
                    names.push(m.get_mapping_name());
                    types.push(m.mapping_type.clone());
                }
            },
            Err(e) => println!("Could not retrieve reference over mapping menus: {}", e),
        }
        (names, types)
    }*/

    /*pub fn update_info(&mut self, ix : usize, new_name : String, new_type : String) {
        if let Ok(mut m_menus) = self.mapping_menus.try_borrow_mut() {
            if let Some(m) = m_menus.get_mut(ix) {

            } else {
                println!("Unable to retrieve mapping menu at index");
            }
        } else {
            println!("Could not recover reference to mapping menus");
        }
    }*/

}

/// LayoutMenu encapsulate the logic of the buttons at the bottom-left
/// that allows changing the plot layout and mappings.
#[derive(Clone)]
pub struct LayoutMenu {
    load_layout_btn : Button,
    new_layout_btn : Button,
    add_mapping_btn : Button,
    //manage_btn : Button,
    remove_mapping_btn : Button,
    layout_stack : Stack
    //manage_mapping_popover : Popover
}

/*fn load_text_content(path : PathBuf)
-> Option<String> {
    if let Ok(mut f) = File::open(path) {
        let mut content = String::new();
        let has_read = f.read_to_string(&mut content);
        if has_read.is_ok() {
            return Some(content);
        } else {
            None
        }
    } else {
        None
    }
}*/

impl LayoutMenu {

    /// The creation of a mapping menu is based on an id naming convention
    /// of passing a prefix identifying the mappping (line, scatter, box, etc)
    /// followed by an element identifier. This convention applies to the enclosing box
    /// (line_box, scatter_box ...) and its constituint widgets (scatter_color_button,
    /// line_color_button) and so on. The builder for each mapping menu must be unique
    /// to avoid aliasing.
    /// Make this mapping_menu::create(.)
    fn create_new_mapping_menu(
        builder : Builder,
        mapping_name : String,
        mapping_type : String,
        data_source : Rc<RefCell<TableEnvironment>>,
        pl_view : Rc<RefCell<PlotView>>,
        properties : Option<HashMap<String, String>>,
        sidebar : PlotSidebar
    ) -> Result<MappingMenu, &'static str> {
        let valid_mappings = ["line", "scatter", "bar", "area", "text", "surface"];
        if !valid_mappings.iter().any(|s| &mapping_type[..] == *s) {
            return Err("Invalid mapping type. Must be line|scatter|bar|area|text|surface");
        }
        let builder = Builder::new_from_file(utils::glade_path("gtk-plots-stack.glade").unwrap());
        let box_name = mapping_type.clone() + "_box";
        let mapping_box : Box = builder.get_object(&box_name).unwrap();
        let combos = MappingMenu::build_combo_columns_menu(
            &builder,
            mapping_type.clone()
        );
        for combo in combos.iter() {
            let data_source = data_source.clone();
            let pl_view = pl_view.clone();
            let curr_name = Rc::new(RefCell::new(mapping_name.clone()));
            let data_source = data_source.clone();
            let sidebar_c = sidebar.clone();
            combo.connect_changed(move |_combo| {
                let data_source = data_source.try_borrow_mut();
                //let pl_view = ;
                let curr_name = curr_name.try_borrow_mut();
                let menus = sidebar_c.mapping_menus.try_borrow_mut();
                let res = match (data_source, pl_view.try_borrow_mut(), curr_name, menus) {
                    (Ok(data), Ok(mut pl), Ok(name), Ok(menus)) => {
                        let m = menus.iter().find(
                            |m| { m.mapping_name == *name });
                        if let Some(m) = m {
                            if let Some(cols) = m.get_selected_cols() {
                                 let res = update_mapping_data(
                                        &data,
                                        name.to_string(),
                                        m.mapping_type.clone(),
                                        cols.clone(),
                                        &mut pl
                                    );
                                    if let Ok(_) = res {
                                        pl.update(&mut UpdateContent::MappingColumn(name.to_string(), cols));
                                    } else {
                                        println!("Error updating data. Column names will not be updated");
                                    }
                                    res
                            } else {
                                Err("No selected cols")
                            }
                        } else {
                            Err("Invalid mapping name")
                        }
                    },
                    _ => {
                        Err("Could not retrieve reference to table environment|plot view|menus")
                    }
                };
                match res {
                    Ok(_) => {
                        Self::update_layout_widgets(
                            sidebar_c.clone(),
                            pl_view.clone()
                        );
                    },
                    Err(e) => {
                        println!("Error updating data : {}", e);
                    }
                }
            });
        }

        let design_widgets = HashMap::new();
        let mut m = MappingMenu{
            mapping_name,
            mapping_type,
            mapping_box,
            combos,
            design_widgets
        };
        m.build_mapping_design_widgets(
            &builder,
            pl_view.clone()
        );

        if let Some(prop) = properties {
            if let Err(e) = m.update_widget_values(prop) {
                println!("{}", e);
            }
        }
        Ok(m)
    }

    fn create_tab_image(m_type : String) -> Image {
        let tab_img_path = String::from("assets/icons/") + &m_type + ".svg";
        Image::new_from_file(&tab_img_path[..])
    }

    fn append_mapping_menu(
        mut m : MappingMenu,
        mappings : Rc<RefCell<Vec<MappingMenu>>>,
        notebook : Notebook,
        plot_view : Rc<RefCell<PlotView>>,
        data_source : Rc<RefCell<TableEnvironment>>,
        pos : Option<usize>
    ) {
        match (plot_view.try_borrow_mut(), data_source.try_borrow_mut(), mappings.try_borrow_mut()) {
            (Ok(mut pl), Ok(source), Ok(mut mappings)) => {
                m.update_available_cols(source.col_names(), &pl);
                match pos {
                    Some(p) => mappings.insert(p, m.clone()),
                    None => mappings.push(m.clone())
                }
                let tab_img = Self::create_tab_image(m.mapping_type.clone());
                notebook.add(&m.get_parent());
                notebook.set_tab_label(&m.get_parent(), Some(&tab_img));
                let npages = notebook.get_children().len() as i32;
                notebook.set_property_page(npages-1);
                notebook.show_all();
                pl.update(&mut UpdateContent::NewMapping(
                    m.mapping_name.to_string(),
                    m.mapping_type.to_string())
                );
            },
            (_,_,Err(e)) => { println!("{}", e); },
            _ => {
                println!("Unable to retrieve mutable reference to plot view|data source");
            }
        }
    }

    fn clear_mappings(
        mappings : Rc<RefCell<Vec<MappingMenu>>>,
        plot_notebook : Notebook
    ) -> Result<(), &'static str> {
        if let Ok(mut mappings) = mappings.try_borrow_mut() {
            for m in mappings.iter() {
                plot_notebook.remove(&m.get_parent());
            }
            mappings.clear();
            Ok(())
        } else {
            Err("Could not fetch mutable reference to mapping menus before clearing them")
        }
    }

    fn update_layout_widgets(
        sidebar : PlotSidebar,
        plot_view : Rc<RefCell<PlotView>>
    ) {
        match plot_view.try_borrow_mut() {
            Ok(pl) => {
                sidebar.design_menu.update(pl.plot_area.design_info());
                sidebar.scale_menus.0.update(pl.plot_area.scale_info("x"));
                sidebar.scale_menus.1.update(pl.plot_area.scale_info("y"));
            },
            _ => {
                panic!("Could not fetch plotview reference to update layout");
            }
        }
    }

    fn build_layout_load_button(
        builder : Builder,
        plot_view : Rc<RefCell<PlotView>>,
        data_source : Rc<RefCell<TableEnvironment>>,
        sidebar : PlotSidebar
    ) -> Button {
        let xml_load_dialog : FileChooserDialog =
            builder.get_object("xml_load_dialog").unwrap();
        let load_btn : Button=
            builder.get_object("load_layout_btn").unwrap();
        {
            let load_btn = load_btn.clone();
            let xml_load_dialog = xml_load_dialog.clone();
            load_btn.connect_clicked(move |_| {
                xml_load_dialog.run();
                xml_load_dialog.hide();
            });
        }
        xml_load_dialog.connect_response(move |dialog, resp|{
            match resp {
                ResponseType::Other(1) => {
                    if let Some(path) = dialog.get_filename() {
                        //let path = f.get_path().unwrap_or(PathBuf::new());
                        //println!("{:?}", path);
                        //if let Some(path) = f {
                        let new_mapping_info = match plot_view.try_borrow_mut() {
                            Ok(mut pl) => {
                                match pl.plot_area.load_layout(path.to_str().unwrap_or("").into()) {
                                    Ok(_) => Some(pl.plot_area.mapping_info()),
                                    Err(e) => { println!("{}", e); None }
                                }
                            },
                            Err(_) => { println!("Could not get reference to Plot widget"); None }
                        };
                        if let Some(new_info) = new_mapping_info {
                            Self::clear_mappings(
                                sidebar.mapping_menus.clone(),
                                sidebar.notebook.clone()
                            ).expect("Error clearing mappings");
                            Self::update_layout_widgets(
                                sidebar.clone(),
                                plot_view.clone()
                            );
                            for m_info in new_info.iter() {
                                let menu = Self::create_new_mapping_menu(
                                    builder.clone(),
                                    m_info.0.clone(),
                                    m_info.1.clone(),
                                    data_source.clone(),
                                    plot_view.clone(),
                                    Some(m_info.2.clone()),
                                    sidebar.clone()
                                );
                                match menu {
                                    Ok(m) => {
                                        Self::append_mapping_menu(
                                            m,
                                            sidebar.mapping_menus.clone(),
                                            sidebar.notebook.clone(),
                                            plot_view.clone(),
                                            data_source.clone(),
                                            None
                                        );
                                    },
                                    Err(e) => { println!("{}", e); return; }
                                }
                            }
                            sidebar.notebook.show_all();
                            //println!("{:?}", mappings);
                        } else {
                            println!("No info to update");
                        }
                    } else {
                        println!("Could not get filename from dialog");
                    }
                },
                _ => { }
            }
        });
        load_btn
    }

    fn selected_mapping_radio(scatter_radio : &RadioButton) -> Option<String> {
        for radio in scatter_radio.get_group() {
            if radio.get_active() {
                if let Some(name) = WidgetExt::get_widget_name(&radio) {
                    return Some(name.as_str().to_string());
                }
            }
        }
        None
    }

    fn set_mapping_radio(scatter_radio : &RadioButton, curr_type : String) {
        for radio in scatter_radio.get_group() {
            if let Some(name) = WidgetExt::get_widget_name(&radio) {
                if name == &curr_type[..] {
                    radio.set_active(true);
                    return;
                }
            }
        }
        println!("Radio not found for informed type");
    }

    fn remove_selected_mapping_page(
        plot_notebook : &Notebook,
        mapping_menus : Rc<RefCell<Vec<MappingMenu>>>,
        plot_view : Rc<RefCell<PlotView>>
    ) {
        let page = plot_notebook.get_property_page() as usize;
        let mapping_ix = page - 3;
        let children = plot_notebook.get_children();
        if let Some(c) = children.get(page) {
            if let Ok(mut menus) = mapping_menus.try_borrow_mut() {
                if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                    menus.remove(mapping_ix);
                    plot_notebook.remove(c);
                    let name = (mapping_ix).to_string();
                    pl_view.update(&mut UpdateContent::RemoveMapping(name));
                } else {
                    println!("Could not get mutable reference to PlotView")
                }
            } else {
                println!("Unable to retrieve mutable reference to mapping_menus when removing page");
            }
        } else {
            println!("Invalid child position");
        }
    }

    /*fn change_manage_popover_state(
        plot_notebook : &Notebook,
        sidebar : &PlotSidebar,
        name_entry : &Entry,
        add_btn : &Button,
        edit_btn : &Button,
        remove_btn : &Button,
        scatter_radio : &RadioButton
    ) {
        let page = plot_notebook.get_property_page();
        let (names, types) = sidebar.used_names_and_types();
        if page > 3 {
            if let Some(n) = names.get( (page - 4) as usize) {
                name_entry.set_text(&n);
            } else {
                println!("No mapping name for page {}", page - 4);
                return;
            }
            if let Some(t) = types.get( (page - 4) as usize) {
                Self::set_mapping_radio(scatter_radio, t.clone());
            } else {
                println!("No mapping type for page {}", page - 4);
                return;
            }
            add_btn.set_sensitive(false);
            edit_btn.set_sensitive(false);
            remove_btn.set_sensitive(true);
        } else {
            let n_children = plot_notebook.get_children().len();
            let new_std_name = format!("mapping_{}", n_children - 3);
            name_entry.set_text(&new_std_name[..]);
            if names.iter().find(|t| { *t == &new_std_name[..] }).is_none() {
                add_btn.set_sensitive(true);
            } else {
                add_btn.set_sensitive(false);
            }
            edit_btn.set_sensitive(false);
            remove_btn.set_sensitive(false);
        }
        name_entry.grab_focus();
    }*/

    /// Add mapping from a type string description, attributing to its
    /// name the number of mappings currently used.
    pub fn add_mapping_from_type(
        mapping_type : &str,
        data_source : Rc<RefCell<TableEnvironment>>,
        plot_view : Rc<RefCell<PlotView>>,
        sidebar : PlotSidebar,
        builder_clone : Builder
    ) {
        let name = if let Ok(menus) = sidebar.mapping_menus.try_borrow() {
            format!("{}", menus.len())
        } else {
            return;
        };
        let menu = LayoutMenu::create_new_mapping_menu(
            builder_clone.clone(),
            name,
            mapping_type.to_string(),
            data_source.clone(),
            plot_view.clone(),
            None,
            sidebar.clone()
        );
        match menu {
            Ok(m) => {
                Self::append_mapping_menu(
                    m,
                    sidebar.mapping_menus.clone(),
                    sidebar.notebook.clone(),
                    plot_view.clone(),
                    data_source.clone(),
                    None
                );
            },
            Err(e) => { println!("{}", e); return; }
        }
    }

    pub fn new_from_builder(
        builder : &Builder,
        plot_view : Rc<RefCell<PlotView>>,
        data_source : Rc<RefCell<TableEnvironment>>,
        sidebar : PlotSidebar
    ) -> Self {
        let load_layout_btn = Self::build_layout_load_button(
            builder.clone(),
            plot_view.clone(),
            data_source.clone(),
            sidebar.clone()
        );
        let new_layout_btn : Button = builder.get_object("layout_new_btn").unwrap();
        let layout_stack = sidebar.layout_stack.clone();
        //layout_stack.add_named(&sidebar.notebook, "layout");
        {
            let layout_stack = layout_stack.clone();
            new_layout_btn.connect_clicked(move |btn| {
                layout_stack.set_visible_child_name("layout");
            });
        }
        let add_mapping_btn : Button = builder.get_object("add_mapping_btn").unwrap();
        let remove_mapping_btn : Button = builder.get_object("remove_mapping_btn").unwrap();
        let clear_layout_btn : Button = builder.get_object("clear_layout_btn").unwrap();
        {
            let layout_stack = layout_stack.clone();
            let plot_view = plot_view.clone();
            let mapping_menus = sidebar.mapping_menus.clone();
            let notebook = sidebar.notebook.clone();
            clear_layout_btn.connect_clicked(move |btn| {
                if let Ok(mut pl_view) = plot_view.try_borrow_mut() {
                    pl_view.update(&mut UpdateContent::Clear(String::from("assets/plot_layout/layout.xml")));
                    layout_stack.set_visible_child_name("empty");
                    if let Ok(mut mappings) = mapping_menus.try_borrow_mut() {
                        mappings.clear();
                    } else {
                        println!("Error retrieving mapping menus");
                    }
                    let children = notebook.get_children();
                    for i in 3..children.len() {
                        if let Some(c) = children.get(i) {
                            notebook.remove(c);
                        } else {
                            println!("Unable to clear notebook");
                        }
                    }
                }
            });
        }

        /*let layout_toolbar : Toolbar = builder.get_object("layout_toolbar").unwrap();
        let img_add = Image::new_from_icon_name(Some("list-add-symbolic"), IconSize::SmallToolbar);
        let img_remove = Image::new_from_icon_name(Some("list-remove-symbolic"), IconSize::SmallToolbar);
        let img_open = Image::new_from_icon_name(Some("document-open-symbolic"), IconSize::SmallToolbar);
        let open_btn : ToolButton = ToolButton::new::<Image>(None, None);
        open_btn.set_icon_name(Some("document-open-symbolic"));
        let add_btn_ : ToolButton = ToolButton::new(Some(&img_add), None);
        let rem_btn : ToolButton = ToolButton::new(Some(&img_remove), None);
        //let edit_btn : ToolButton = ToolButton::new(Some(&img_edit), None);
        layout_toolbar.insert(&open_btn, 0);
        layout_toolbar.insert(&add_btn_, 1);
        layout_toolbar.insert(&rem_btn, 2);
        //layout_toolbar.insert(&add_btn, 0);
        layout_toolbar.show_all();*/

        let add_mapping_popover : Popover = builder.get_object("add_mapping_popover").unwrap();
        add_mapping_popover.set_relative_to(Some(&add_mapping_btn));
        let upper_mapping_toolbar : Toolbar = builder.get_object("upper_mapping_toolbar").unwrap();
        let lower_mapping_toolbar : Toolbar = builder.get_object("lower_mapping_toolbar").unwrap();
        let toolbars = [upper_mapping_toolbar, lower_mapping_toolbar];
        let mapping_names = vec![
            String::from("line"),
            String::from("scatter"),
            String::from("bar"),
            String::from("text"),
            String::from("area"),
            String::from("surface")
        ];
        let iter_names = mapping_names.iter();
        for (i, mapping) in iter_names.enumerate() {
            //let mut m_name = String::from(&mapping[0..1].to_uppercase());
            //m_name += &mapping[1..];
            let img = Image::new_from_file(&(String::from("assets/icons/") +  mapping + ".svg"));
            let btn : ToolButton = ToolButton::new(Some(&img), None);
            toolbars[i / 3].insert(&btn, (i % 3) as i32);
            let m = mapping.clone();
            let add_mapping_popover = add_mapping_popover.clone();
            let builder = builder.clone();
            let data_source = data_source.clone();
            let plot_view = plot_view.clone();
            let sidebar = sidebar.clone();
            let remove_mapping_btn = remove_mapping_btn.clone();
            btn.connect_clicked(move |_btn| {
                Self::add_mapping_from_type(
                    &m[..],
                    data_source.clone(),
                    plot_view.clone(),
                    sidebar.clone(),
                    builder.clone()
                );
                add_mapping_popover.hide();
                remove_mapping_btn.set_sensitive(true);
            });
        }
        toolbars.iter().for_each(|t| t.show_all() );
        add_mapping_btn.connect_clicked(move|_btn| {
            add_mapping_popover.show();
        });

        {
            let plot_notebook = sidebar.notebook.clone();
            //let sidebar = sidebar.clone();
            //let name_entry = name_entry.clone();
            //let edit_btn = edit_btn.clone();
            //let add_btn = add_btn.clone();
            let remove_mapping_btn = remove_mapping_btn.clone();
            //let scatter_radio = scatter_radio.clone();
            //plot_notebook.clone().connect_switch_page(move |_nb, _wid, _page| {
            plot_notebook.clone().connect_switch_page(move |_nb, wid, page| {
                //let page = plot_notebook.get_property_page();
                println!("{}", page);
                if page > 2 {
                    remove_mapping_btn.set_sensitive(true);
                } else {
                    remove_mapping_btn.set_sensitive(false);
                }
                //true
                //if manage_mapping_popover.is_visible() {
                /*Self::change_manage_popover_state(
                    &plot_notebook,
                    &sidebar,
                    &name_entry,
                    &add_btn,
                    &edit_btn,
                    &remove_btn,
                    &scatter_radio
                );*/
                //}
            });
        }

        /*{
            for btn in scatter_radio.get_group() {
                let edit_btn = edit_btn.clone();
                // let scatter_radio = scatter_radio.clone();
                let sidebar = sidebar.clone();
                let plot_notebook = sidebar.notebook.clone();
                let name_entry = name_entry.clone();
                let remove_btn = remove_btn.clone();
                btn.connect_toggled(move |radio| {
                    //if let Some(selected_type) = Self::selected_mapping_radio(&scatter_radio) {
                    let (names, types) = sidebar.used_names_and_types();
                    let sel_page = plot_notebook.get_property_page();
                    if sel_page > 3 {
                        let sel_type = types[(sel_page - 4) as usize].clone();
                        let sel_name = names[(sel_page - 4) as usize].clone();
                        if let Some(new_type) = WidgetExt::get_widget_name(radio) {
                            if new_type != sel_type && !edit_btn.is_sensitive() {
                                edit_btn.set_sensitive(true);
                                remove_btn.set_sensitive(false);
                            } else {
                                if let Some(name) = name_entry.get_text().map(|t| t.to_string()) {
                                    if name == sel_name && edit_btn.is_sensitive() {
                                        edit_btn.set_sensitive(false);
                                    }
                                    remove_btn.set_sensitive(true);
                                }
                            }
                        }
                    }
                    //}
                });
            }
        }*/

        /*{
            let manage_mapping_popover = manage_mapping_popover.clone();
            let plot_notebook = sidebar.notebook.clone();
            let sidebar = sidebar.clone();
            let name_entry = name_entry.clone();
            let edit_btn = edit_btn.clone();
            let add_btn = add_btn.clone();
            let remove_btn = remove_btn.clone();
            let scatter_radio = scatter_radio.clone();
            manage_btn.connect_clicked(move |_| {
                Self::change_manage_popover_state(
                    &plot_notebook,
                    &sidebar,
                    &name_entry,
                    &add_btn,
                    &edit_btn,
                    &remove_btn,
                    &scatter_radio
                );
                manage_mapping_popover.show();
            });
        }*/

        /*{
            let manage_mapping_popover = manage_mapping_popover.clone();
            let sidebar = sidebar.clone();
            let plot_view = plot_view.clone();
            let scatter_radio = scatter_radio.clone();
            let name_entry = name_entry.clone();
            let plot_notebook = sidebar.notebook.clone();
            let builder = builder.clone();
            let data_source = data_source.clone();
            edit_btn.connect_clicked(move |_|{
                let menu = match plot_view.try_borrow_mut() {
                    Ok(mut pl_view) => {
                        let new_type = Self::selected_mapping_radio(&scatter_radio);
                        let new_name = name_entry.get_text().map(|txt| txt.to_string());
                        // here sidebar.mapping_menus is NOT mutably borrowed.
                        /*if let Ok(m) = sidebar.mapping_menus.try_borrow() {
                            println!("Could borrow here");
                        } else {
                            println!("Could not!");
                        }*/
                        let (names, types) = sidebar.used_names_and_types();
                        let sel_ix = (plot_notebook.get_property_page() - 4) as usize;
                        let old_name = names.get(sel_ix);
                        let old_type = types.get(sel_ix);
                        match (old_name, old_type, new_name, new_type) {
                            (Some(old_n), Some(old_t), Some(new_n), Some(new_t)) => {
                                if &new_n != old_n || &new_t != old_t {
                                    pl_view.update(&mut UpdateContent::RemoveMapping(old_n.clone()));
                                    Self::create_new_mapping_menu(
                                        builder.clone(),
                                        new_n,
                                        new_t,
                                        data_source.clone(),
                                        plot_view.clone(),
                                        None,
                                        sidebar.clone()
                                    )
                                } else {
                                    Err("No update requested by user")
                                }
                            },
                            _ => {
                                Err("Unable to retrieve (type, name) pair when updating mapping.")
                            }
                        }
                    },
                    Err(_) => {
                        Err("Unable to retrieve mutable reference to plot view")
                    }
                };
                match menu {
                    Ok(m) => {
                        let page_pos = plot_notebook.get_property_page();
                        let m_parent = m.get_parent();
                        Self::remove_selected_mapping_page(
                            &plot_notebook,
                            sidebar.mapping_menus.clone()
                        );
                        Self::append_mapping_menu(
                            m.clone(),
                            sidebar.mapping_menus.clone(),
                            plot_notebook.clone(),
                            plot_view.clone(),
                            data_source.clone(),
                            Some( (page_pos - 4) as usize)
                        );
                        // if let Some(mut m_menus) = mappings.try_borrow_mut() {
                        //    let last_pos = m_menus.len() - 1;
                        //    m_menus.swap( (page_pos - 4) as usize, last_pos);
                        // }
                        plot_notebook.reorder_child(&m_parent, Some(page_pos as u32));
                        name_entry.set_text(&m.get_mapping_name());
                        Self::set_mapping_radio(&scatter_radio, m.mapping_type.clone());
                        // for m in mappings.borrow().iter() {
                        //    println!("{}; {}", m.get_mapping_name(), m.mapping_type);
                        // }
                        plot_notebook.show_all();
                    },
                    Err(e) => {
                        println!("Unable to create new mapping menu: {}", e);
                    }
                }
                manage_mapping_popover.hide();
            });
        }*/

        {
            //let manage_mapping_popover = manage_mapping_popover.clone();
            let sidebar = sidebar.clone();
            let plot_view = plot_view.clone();
            //let name_entry = name_entry.clone();
            let plot_notebook = sidebar.notebook.clone();
            remove_mapping_btn.connect_clicked(move |_| {
                //let name = name_entry.get_text().map(|txt| txt.to_string());
                //match plot_view.try_borrow_mut() {
                    //Ok(mut pl_view) =>  {
                    Self::remove_selected_mapping_page(
                        &plot_notebook,
                        sidebar.mapping_menus.clone(),
                        plot_view.clone()
                    );
                    //},
                    //_ => {
                    //    println!("Unable to retrieve reference to plot or to mapping name when deleting.");
                   // }
                //}
                plot_notebook.show_all();
                //manage_mapping_popover.hide();
            });
        }

        /*{
            let sidebar = sidebar.clone();
            let remove_btn = remove_btn.clone();
            let add_btn = add_btn.clone();
            let plot_notebook = sidebar.notebook.clone();
            name_entry.connect_key_release_event(move |entry, _ev_key| {
                if let Some(txt) = entry.get_text().map(|txt| txt.to_string()) {
                    let (names, _) = sidebar.used_names_and_types();
                    let sel_page = plot_notebook.get_property_page();
                    let has_name = names.iter().find(|t| { *t == &txt[..] }).is_some();
                    if sel_page > 3 {
                        let name_match = txt == names[(sel_page - 4) as usize];
                        match (has_name, name_match) {
                            (true, true) => {
                                remove_btn.set_sensitive(true);
                                add_btn.set_sensitive(false);
                                edit_btn.set_sensitive(false);
                            },
                            _ => {
                                add_btn.set_sensitive(true);
                                edit_btn.set_sensitive(true);
                                remove_btn.set_sensitive(false);
                            }
                        }
                    } else {
                        match has_name {
                            true => add_btn.set_sensitive(false),
                            false => add_btn.set_sensitive(true)
                        }
                        remove_btn.set_sensitive(false);
                        edit_btn.set_sensitive(false);
                    }
                }
                glib::signal::Inhibit(false)
            });
        }*/

        /*{
            let builder_clone = builder.clone();
            let name_entry = name_entry.clone();
            let manage_mapping_popover = manage_mapping_popover.clone();
            add_btn.connect_clicked(move |_btn| {
                let mapping_type = Self::selected_mapping_radio(&scatter_radio);
                match mapping_type {
                    Some(mapping_type) => {
                        let mapping_name = name_entry.get_text().map(|t| t.to_string());
                        if let Some(name) = mapping_name {
                            let menu = LayoutMenu::create_new_mapping_menu(
                                builder_clone.clone(),
                                name.as_str().to_owned(),
                                mapping_type.as_str().to_owned(),
                                data_source.clone(),
                                plot_view.clone(),
                                None,
                                sidebar.clone()
                            );
                            match menu {
                                Ok(m) => {
                                    Self::append_mapping_menu(
                                        m,
                                        sidebar.mapping_menus.clone(),
                                        sidebar.notebook.clone(),
                                        plot_view.clone(),
                                        data_source.clone(),
                                        None
                                    );
                                },
                                Err(e) => { println!("{}", e); return; }
                            }
                        } else {
                            println!("Could not retrieve mapping name");
                        }
                    },
                    _ => { println!("Unknown mapping type"); }
                }
                manage_mapping_popover.hide();
            });
        }*/
        Self {
            load_layout_btn,
            add_mapping_btn,
            new_layout_btn,
            remove_mapping_btn,
            layout_stack
            //manage_mapping_popover
        }
    }

}

