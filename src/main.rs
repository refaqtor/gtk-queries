use gtk::*;
use gio::prelude::*;
use std::env::{self, args};
use std::rc::Rc;
use std::cell::{RefCell, RefMut};
use std::fs::File;
use std::io::Write;
use std::io::Read;
use std::collections::HashMap;
// use gtk_plots::conn_popover::{ConnPopover, TableDataSource};
use std::path::PathBuf;
// use sourceview::*;
use std::ffi::OsStr;
use gdk::ModifierType;
use gdk::{self, enums::key};
use tables::{self, environment_source::EnvironmentSource, TableEnvironment, button::TableChooser, sql::SqlListener};
mod conn_popover;
use conn_popover::*;
use sourceview::*;
use std::boxed;
use std::process::Command;
use gtk::prelude::*;
use gtk_queries::{utils, table_widget::TableWidget, table_notebook::TableNotebook };
use nlearn::table::Table;
use gtk_queries::status_stack::*;

#[derive(Clone)]
pub struct QueriesApp {
    exec_btn : Button,
    view : sourceview::View,
    tables_nb : TableNotebook,
    // file_btn : FileChooserButton,
    header : HeaderBar,
    // unsaved_dialog : Dialog,
    // new_file : Rc<RefCell<bool>>,
    // unsaved_changes : Rc<RefCell<bool>>,
    // save_dialog : Dialog,
    popover : ConnPopover,
    table_env : Rc<RefCell<TableEnvironment>>,
    query_popover : Popover,
    query_toggle : ToggleButton //,
    //ws_toggle : ToggleButton

    // old_source_content : Rc<RefCell<String>>
}

pub fn set_tables(
    table_env : &TableEnvironment,
    tables_nb : &mut TableNotebook
) {
    tables_nb.clear();
    let all_tbls = table_env.all_tables_as_rows();
    if all_tbls.len() == 0 {
        tables_nb.add_page("application-exit",
            None, Some("No queries"), None);
    } else {
        tables_nb.clear();
        for t_rows in all_tbls {
            let nrows = t_rows.len();
            //println!("New table with {} rows", nrows);
            if nrows > 0 {
                let ncols = t_rows[0].len();
                let name = format!("({} x {})", nrows - 1, ncols);
                tables_nb.add_page("network-server-symbolic",
                    Some(&name[..]), None, Some(t_rows));
            } else {
                println!("No rows to display");
            }
        }
    }
}

pub fn send_query_and_wait(
    sql : String,
    tbl_env : &mut TableEnvironment,
    view : &sourceview::View,
    nb : &TableNotebook
) {
    tbl_env.send_query(sql);
    view.set_sensitive(false);
    nb.nb.set_sensitive(false);
}

pub fn update_queries(
    tbl_env : &mut TableEnvironment,
    view : &sourceview::View,
    nb : &TableNotebook
) {
    if let Some(buffer) = view.get_buffer() {
        let text : Option<String> = match buffer.get_selection_bounds() {
            Some((from,to,)) => {
                from.get_text(&to).map(|txt| txt.to_string())
            },
            None => {
                buffer.get_text(
                    &buffer.get_start_iter(),
                    &buffer.get_end_iter(),
                    true
                ).map(|txt| txt.to_string())
            }
        };
        if let Some(txt) = text {
            send_query_and_wait(txt, tbl_env, view, nb);
        }
    } else {
        println!("Could not retrieve text buffer");
    }
}

impl QueriesApp {

    pub fn new_from_builder(builder : &Builder) -> Self {
        let header : HeaderBar =
            builder.get_object("header").unwrap();
        let tables_nb = TableNotebook::new(&builder);
        let exec_btn : Button =
            builder.get_object("exec_btn").unwrap();
        let view : sourceview::View =
            builder.get_object("query_source").unwrap();
        let lang_manager = LanguageManager::get_default().unwrap();
        let buffer = view.get_buffer().unwrap()
            .downcast::<sourceview::Buffer>().unwrap();
        let lang = lang_manager.get_language("sql").unwrap();
        buffer.set_language(Some(&lang));
        let env_source = EnvironmentSource::File("".into(),"".into());
        let table_env = TableEnvironment::new(env_source);
        let table_env = Rc::new(RefCell::new(table_env));
        let conn_btn : Button = builder.get_object("conn_btn").unwrap();
        let popover_path = utils::glade_path("conn-popover.glade")
            .expect("Could not open glade path");
        let popover = ConnPopover::new_from_glade(conn_btn, &popover_path[..]);
        //let file_btn : FileChooserButton =
        //    builder.get_object("file_btn").unwrap();
        let table_popover : Popover =
            builder.get_object("table_popover").unwrap();
        let query_stack : Stack = builder.get_object("query_stack").unwrap();
        let status_stack = StatusStack::new(query_stack, tables_nb.nb.clone().upcast::<Widget>());
        popover.hook_signals(table_env.clone(), status_stack.clone());

        //let ops_stack : Stack =
        //    builder.get_object("ops_stack").unwrap();
        /*let ws_toggle : ToggleButton =
            builder.get_object("ws_toggle").unwrap();
        //let query_toggle : ToggleButton =
        //    builder.get_object("query_toggle").unwrap();

        {
            let table_popover = table_popover.clone();
            ws_toggle.connect_toggled(move |toggle| {
                if toggle.get_active() {
                    table_popover.show();
                } else {
                    table_popover.hide();
                    //filter_popover.hide();
                }
            });
        }*/

        /*{
            let ws_toggle = ws_toggle.clone();
            let table_popover = table_popover.clone();
            table_popover.connect_closed(move |_popover| {
                ws_toggle.set_active(false);
            });
        }*/

        let new_db_dialog : FileChooserDialog =
            builder.get_object("new_db_dialog").unwrap();
        {
            let new_db_btn : Button =
                builder.get_object("new_db_btn").unwrap();
            let new_db_dialog = new_db_dialog.clone();
            new_db_btn.connect_clicked(move |_btn| {
                new_db_dialog.run();
                new_db_dialog.hide();
            });
        }

        {
            let t_env = table_env.clone();
            new_db_dialog.connect_response(move |dialog, resp|{
                match resp {
                    ResponseType::Other(1) => {
                        if let Some(path) = dialog.get_filename() {
                            if let (Ok(mut t_env), Some(p_str)) = (t_env.try_borrow_mut(), path.to_str()) {
                                let res = t_env.update_source(
                                    EnvironmentSource::SQLite3((
                                        Some(p_str.into()),
                                        String::from(""))
                                    ),
                                    true
                                );
                                match res {
                                    Ok(_) => { },
                                    Err(s) => println!("{}", s)
                                }
                            } else {
                                println!("Could not acquire mutable reference t_env/path not convertible");
                            }
                        } else {
                            println!("No filename informed");
                        }
                    },
                    _ => { }
                }
            });
        }

        let csv_upload_btn : Button =
            builder.get_object("csv_upload_btn").unwrap();
        //let code_upload_btn : Button =
        //    builder.get_object("new_db_btn").unwrap();
        let csv_upload_dialog : FileChooserDialog =
            builder.get_object("csv_upload_dialog").unwrap();
        {
            let csv_upload_dialog = csv_upload_dialog.clone();
            csv_upload_btn.connect_clicked(move |_btn| {
                csv_upload_dialog.run();
                csv_upload_dialog.hide();
            });
        }

        {
            let t_env = table_env.clone();
            let view = view.clone();
            let nb = tables_nb.clone();
            csv_upload_dialog.connect_response(move |dialog, resp|{
                match resp {
                    ResponseType::Other(1) => {
                        if let Some(path) = dialog.get_filename() {

                        } else {
                            println!("Filename not provided for dialog")
                        }
                    },
                    _ => { }
                }
            });
        }

        // let tables_nb_c = tables_nb.clone();
        //let mut table_chooser = TableChooser::new(file_btn, table_env.clone());
        /*let table_info_box : Box = builder.get_object("table_info_box").unwrap();
        {
            let table_env = table_env.clone();
            //let tables_nb = tables_nb.clone();
            table_chooser.append_cb(boxed::Box::new(move |btn| {
                /*if let Ok(t_env) = table_env.try_borrow_mut() {
                    set_tables(&t_env, &mut tables_nb.clone());
                } else {
                    println!("Unable to get reference to table env");
                }*/
                if let Ok(t_env) = table_env.try_borrow_mut() {
                    for w in table_info_box.get_children() {
                        table_info_box.remove(&w);
                    }
                    if let Some(info) = t_env.table_names_as_hash() {
                        for (table, cols) in info.iter() {
                            let exp = Expander::new(Some(&table));
                            let exp_content = Box::new(Orientation::Vertical, 0);
                            for c in cols {
                                exp_content.add(&Label::new(Some(&c.0)));
                            }
                            exp.add(&exp_content);
                            table_info_box.add(&exp);
                        }
                        table_info_box.show_all();
                    } else {
                        println!("Could not get table info as hash");
                    }
                }

            }));
        }*/

        // let sql_listener = Rc::new(RefCell::new(SqlListener::launch()));
        //let table_env_c = table_env.clone();
        let query_popover : Popover =
            builder.get_object("query_popover").unwrap();
        let query_toggle : ToggleButton =
            builder.get_object("query_toggle").unwrap();
        let queries_app = QueriesApp{
            exec_btn : exec_btn, view : view, tables_nb : tables_nb.clone(), header : header,
            popover : popover, table_env : table_env.clone(), query_popover : query_popover,
            query_toggle : query_toggle //, ws_toggle : ws_toggle
        };
        let queries_app_c = queries_app.clone();
        // let sql_listener_c = sql_listener.clone();
        let table_env_c = queries_app.clone().table_env.clone();
        let view_c = queries_app_c.view.clone();
        let tables_nb_c = queries_app.clone().tables_nb.clone();
        view_c.connect_key_press_event(move |view, ev_key| {
            if ev_key.get_state() == gdk::ModifierType::CONTROL_MASK && ev_key.get_keyval() == key::Return {
                match table_env_c.try_borrow_mut() {
                    Ok(mut env) => {
                        update_queries(&mut env, &view.clone(), &tables_nb_c.clone());
                    },
                    _ => { println!("Error recovering references"); }
                }
                glib::signal::Inhibit(true)
            } else {
                glib::signal::Inhibit(false)
            }
        });
        // Check if there is a SQL answer before setting the widgets to sensitive again.
        //let sql_listener_c = sql_listener.clone();
        {
            let status_stack = status_stack.clone();
            let queries_app_c = queries_app.clone();
            let view_c = queries_app_c.view.clone();
            let tables_nb_c = queries_app_c.tables_nb.clone();
            let tbl_env_c = table_env.clone();
            gtk::timeout_add(16, move || {
                if !view_c.is_sensitive() {
                    if let Ok(mut t_env) = tbl_env_c.try_borrow_mut() {
                        if let Some(last_cmd) = t_env.last_commands().last() {
                            if &last_cmd[..] == "select" {
                                match t_env.maybe_update_from_query_results() {
                                    Some(Ok(_)) => {
                                        set_tables(&t_env, &mut tables_nb_c.clone());
                                        status_stack.update(Status::Ok);
                                    },
                                    Some(Err(e)) => {
                                        println!("{}", e);
                                        status_stack.update(Status::SqlErr(e));
                                    },
                                    None => { }
                                }
                            } else {
                                match t_env.result_last_statement() {
                                    Some(Ok(ans)) => {
                                        status_stack.update(Status::StatementExecuted(ans));
                                    },
                                    Some(Err(e)) => {
                                        println!("{}", e);
                                        status_stack.update(Status::SqlErr(e));
                                    },
                                    None => { }
                                }
                            }
                        } else {
                            println!("Unable to retrieve last command");
                            return glib::source::Continue(true);
                        }
                        view_c.set_sensitive(true);
                        tables_nb_c.nb.set_sensitive(true);
                    }
                }
                glib::source::Continue(true)
            });
        }
        let table_menu_c = queries_app.query_popover.clone();
        queries_app.query_toggle.connect_toggled(move |toggle| {
            if toggle.get_active() {
                table_menu_c.show();
            } else {
                table_menu_c.hide();
            }
        });

        let query_toggle_c = queries_app.query_toggle.clone();
        queries_app.query_popover.connect_closed(move |_popover| {
            query_toggle_c.set_active(false);
        });
        {
            let queries_app = queries_app.clone();
            let table_env_c = table_env.clone();
            let view_c = queries_app_c.view.clone();
            let tables_nb_c = queries_app_c.tables_nb.clone();
            queries_app.clone().exec_btn.connect_clicked(move |btn| {
                if let Ok(mut env) = table_env_c.try_borrow_mut() {
                    update_queries(&mut env, &view_c, &tables_nb_c);
                } else {
                    println!("Failed to acquire lock");
                }
            });
        }

        {
            let tables_nb = tables_nb.clone();
            let tbl_env = table_env.clone();
            let csv_btn : Button =
                builder.get_object("csv_btn").unwrap();
            let save_dialog : FileChooserDialog =
                builder.get_object("save_dialog").unwrap();
            save_dialog.connect_response(move |dialog, resp|{
                match resp {
                    ResponseType::Other(1) => {
                        if let Some(path) = dialog.get_filename() {
                            if let Some(ext) = path.as_path().extension().map(|ext| ext.to_str().unwrap_or("")) {
                                if let Ok(t) = tbl_env.try_borrow() {
                                    match ext {
                                        "db" | "sqlite" | "sqlite3" => {
                                            t.try_backup(path);
                                        },
                                        _ => {
                                            if let Ok(mut f) = File::create(path) {
                                                let idx = tables_nb.get_page_index();
                                                if let Some(content) = t.get_text_at_index(idx) {
                                                    let _ = f.write_all(&content.into_bytes());
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    println!("Unable to get reference to table environment");
                                }
                            }
                        }
                    },
                    _ => { }
                }
            });
            csv_btn.connect_clicked(move |btn| {
                save_dialog.run();
                save_dialog.hide();
            });
        }

        queries_app
    }

    fn check_active_selection(&self) {
        if let Some(buf) = self.view.get_buffer() {

            /*if buf.get_has_selection() {
                self.exec_btn.set_sensitive(true);
            } else {
                self.exec_btn.set_sensitive(false);
            }*/

        }
    }

    /*fn run_query(popover : &mut ConnPopover, buffer : &TextBuffer) {
        let mut query = String::new();
        if let Some((from,to,)) = buffer.get_selection_bounds() {
            if let Some(txt) = from.get_text(&to) {
                query = txt.to_string();
            }
        }
        if query.len() > 0 {
            popover.parse_sql(query);
            popover.try_run_all();
        }
        // TODO Update notebook here with query results
    }*/

}

fn build_ui(app: &gtk::Application) {
    let path = utils::glade_path("gtk-queries.glade").expect("Failed to load glade file");
    let builder = Builder::new_from_file(path);
    let win : Window = builder.get_object("main_window")
        .expect("Could not recover window");

    let queries_app = QueriesApp::new_from_builder(&builder);

    {
        let toggle_q = queries_app.query_toggle.clone();
        //let toggle_w = queries_app.ws_toggle.clone();
        let view = queries_app.view.clone();
        win.connect_key_release_event(move |win, ev_key| {
            if ev_key.get_state() == gdk::ModifierType::MOD1_MASK {
                if ev_key.get_keyval() == key::q {
                    if toggle_q.get_active() {
                        toggle_q.set_active(false);
                    } else {
                        toggle_q.set_active(true);
                        view.grab_focus();
                    }
                    return glib::signal::Inhibit(true)
                }
                if ev_key.get_keyval() == key::w {
                    //if toggle_w.get_active() {
                    //    toggle_w.set_active(false);
                    //} else {
                    //    toggle_w.set_active(true);
                    //}
                    return glib::signal::Inhibit(true)
                }
                return glib::signal::Inhibit(false)
            } else {
                glib::signal::Inhibit(false)
            }
        });
    }

    win.set_application(Some(app));

    win.show_all();
}

fn main() {
    gtk::init();

    // Required for GtkSourceView initialization from glade
    let _ = View::new();

    let app = gtk::Application::new(
        Some("com.github.limads.gtk-plots"),
        Default::default())
    .expect("Could not initialize Gtk");

    app.connect_activate(|app| {
        build_ui(app);
    });

    app.run(&args().collect::<Vec<_>>());
}

// Change GtkSourceView on file set
/*{
    let view = view.clone();
    // let new_file = new_file.clone();
    // let header = header.clone();
    // let unsaved_dialog = unsaved_dialog.clone();
    let unsaved_changes = unsaved_changes.clone();
    file_btn.connect_file_set(move |btn| {
        let buffer = view.get_buffer();
        let new_file = new_file.try_borrow_mut();
        let unsaved = unsaved_changes.try_borrow_mut();
        match (buffer, new_file, unsaved) {
            (Some(mut buf), Ok(mut new_f), Ok(mut unsaved)) => {
                let from = buf.get_start_iter();
                let to = buf.get_end_iter();
                let empty_buf =  from == to;
                if (*new_f && !empty_buf) || *unsaved {
                    match unsaved_dialog.run() {
                        ResponseType::Other(0) => {
                            unsaved_dialog.hide();
                        },
                        ResponseType::Other(1) => {
                            buf.set_text("");
                            let ok = QueriesApp::load_file_to_buffer(
                                btn.clone(),
                                buf,
                                header.clone(),
                                *new_f,
                                *unsaved
                            ).is_ok();
                            if ok {
                                *new_f = false;
                                *unsaved = false;
                            }
                            unsaved_dialog.hide();
                        },
                        _ => { }
                    }
                }
            },
            _ => { println!("Unavailable reference"); }
        }
    });*/

// SourceView key release
/*{
let exec_btn = queries_app.exec_btn.clone();
let queries_app = queries_app.clone();
queries_app.clone().view.connect_key_release_event(move |view, ev| {
    queries_app.check_active_selection();
    if let Some(buf) = queries_app.view.get_buffer() {
        let from = &buf.get_start_iter();
        let to = &buf.get_end_iter();
        let old = queries_app.old_source_content.borrow();
        let unsaved = queries_app.unsaved_changes.borrow();
        if let Some(txt) = buf.get_text(from, to,true) {
            if *unsaved && txt != *old {
                let mut subtitle : String =
                    queries_app.header.get_subtitle()
                    .and_then(|s| Some(s.to_string()) )
                    .unwrap_or("".into());
                subtitle += "*";
                queries_app.header.set_subtitle(
                    Some(&subtitle[..]));
            }
        }
    }
glib::signal::Inhibit(false)
});
}*/

// Button release on GtkSourceView
/*
{
    let queries_app = queries_app.clone();
    queries_app.clone()
    .view.connect_button_release_event(move |view, ev| {
        queries_app.check_active_selection();
        glib::signal::Inhibit(false)
    });
}
*/

// Key press on GtkSourceView
/*{
    let queries_app = queries_app.clone();
    queries_app.clone()
    .view.connect_key_press_event(move |view, ev_key| {
        // check gdkkeysyms.h
        println!("{:?}", ev_key.get_keyval());

        if ev_key.get_state() == gdk::ModifierType::CONTROL_MASK {
            if ev_key.get_keyval() == 115 {
                println!("must save now");
            }
        }

        match ev_key.get_keyval() {
            // s, i.e. CTRL+s because plain s is
            // captured and inhibited
            //if
            0x073 => {
                // queries_app.try_save_file();
            },
            // space
            0x020 => {
            }
            _ => { }
        };
        glib::signal::Inhibit(false)
    });
}*/

