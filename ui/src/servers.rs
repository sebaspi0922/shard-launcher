//! Servidores (servers_page.dart, server_dialog.dart): lista reordenable con
//! estado en vivo (MOTD, jugadores, ping y favicon), instancia y cuenta por
//! servidor, «Entrar», añadir, editar, quitar, copiar la dirección e
//! importar de `servers.dat`.
//!
//! La lista y los estados los tiene `Controller` (Inicio también los usa);
//! aquí se pintan, se atienden los callbacks de `ServersState` y se pide al
//! núcleo con un [`ServerActions`], que contesta con [`post`].

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel};

use crate::accounts::Accounts;
use crate::content::{instance_badge, instance_option, plain_option};
use crate::controller::Controller;
use crate::errors::Failure;
use crate::home::{self, Server, Status};
use crate::{MainWindow, ServerCard, ServerMenu, ServersState, i18n};

/// `host`, `host:puerto` o un nombre con puntos y guiones (`_addressRe`).
pub fn valid_address(a: &str) -> bool {
    let a = a.trim();
    !a.is_empty() && a.chars().all(|c| c.is_ascii_alphanumeric() || ".-_[]:".contains(c))
}

/// Lo que se guarda desde el diálogo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    /// `None` = servidor nuevo.
    pub id: Option<String>,
    pub name: String,
    pub address: String,
    pub instance_id: Option<String>,
    pub account_id: Option<String>,
}

/// Lo que la pantalla pide fuera de la UI.
pub trait ServerActions {
    /// → [`Servers::saved`]; si va bien, la lista se vuelve a leer.
    fn save(&self, draft: &Draft);
    /// → [`Servers::removed`].
    fn remove(&self, id: &str, name: &str);
    /// Reordena y guarda → [`Servers::moved`].
    fn move_to(&self, id: &str, index: usize);
    /// `servers.dat` de una instancia → [`Servers::imported`].
    fn import(&self, instance_id: &str, instance_name: &str);
    /// Vuelve a leer la lista y a consultar los estados.
    fn reload(&self);
}

thread_local! {
    static SERVERS: RefCell<Option<Rc<Servers>>> = const { RefCell::new(None) };
}

/// Ejecuta `f` en el hilo de la UI con el controlador de Servidores.
pub fn post(f: impl FnOnce(&Servers) + Send + 'static) {
    let r = slint::invoke_from_event_loop(move || {
        SERVERS.with(|s| {
            if let Some(s) = s.borrow().as_ref() {
                f(s);
            }
        })
    });
    if let Err(e) = r {
        tracing::debug!(error = %e, "la UI ya no está");
    }
}

/// Diálogo abierto.
#[derive(Debug, Clone, Default)]
struct Dialog {
    open: bool,
    editing: Option<String>,
    instance: Option<String>,
    account: Option<String>,
    busy: bool,
    touched: bool,
    error: Option<Failure>,
}

pub struct Servers {
    ui: slint::Weak<MainWindow>,
    shell: Rc<Controller>,
    accounts: Rc<Accounts>,
    actions: Box<dyn ServerActions>,
    dialog: RefCell<Dialog>,
    /// Consultando los estados (botón «Actualizar» con su ruedita).
    pinging: Cell<bool>,
    /// Orden mientras se guarda un arrastre (se ve ya movido).
    order: RefCell<Option<Vec<String>>>,
    cards: Rc<VecModel<ServerCard>>,
}

impl Servers {
    pub fn new(ui: &MainWindow, shell: Rc<Controller>, accounts: Rc<Accounts>, actions: Box<dyn ServerActions>) -> Rc<Self> {
        let me = Rc::new(Self {
            ui: ui.as_weak(),
            shell,
            accounts,
            actions,
            dialog: RefCell::default(),
            pinging: Cell::new(false),
            order: RefCell::default(),
            cards: Rc::new(VecModel::default()),
        });
        SERVERS.with(|s| *s.borrow_mut() = Some(me.clone()));
        let st = ui.global::<ServersState>();
        st.set_items(ModelRc::from(me.cards.clone()));
        macro_rules! on {
            ($set:ident, | $c:ident $(, $arg:ident)* | $body:expr) => {{
                let weak = Rc::downgrade(&me);
                st.$set(move |$($arg),*| {
                    if let Some($c) = weak.upgrade() {
                        $body;
                    }
                });
            }};
        }
        on!(on_shown, |c| c.shown());
        on!(on_refresh, |c| c.refresh_status());
        on!(on_add, |c| c.open_dialog(None));
        on!(on_edit, |c, id| c.open_dialog(Some(id.to_string())));
        on!(on_join, |c, id| c.shell.join(&id));
        on!(on_copy, |c, id| c.copy(&id));
        on!(on_remove, |c, id| c.remove(&id));
        on!(on_import, |c, id| c.import(&id));
        on!(on_move, |c, id, index| c.move_to(&id, index.max(0) as usize));
        on!(on_dialog_instance_changed, |c, v| {
            c.dialog.borrow_mut().instance = Some(v.to_string()).filter(|s| !s.is_empty());
            c.refresh();
        });
        on!(on_dialog_account_changed, |c, v| {
            c.dialog.borrow_mut().account = Some(v.to_string()).filter(|s| !s.is_empty());
            c.refresh();
        });
        on!(on_dialog_save, |c| c.save());
        on!(on_dialog_close, |c| c.close_dialog());
        on!(on_address_edited, |c| c.refresh_dialog_error());
        me.refresh();
        me
    }

    fn lang(&self) -> String {
        self.shell.lang()
    }

    fn server(&self, id: &str) -> Option<Server> {
        self.shell.servers().unwrap_or_default().into_iter().find(|s| s.id == id)
    }

    /// Lista en el orden que se ve (con un arrastre a medio guardar).
    fn list(&self) -> Vec<Server> {
        let mut list = self.shell.servers().unwrap_or_default();
        if let Some(order) = self.order.borrow().as_ref() {
            list.sort_by_key(|s| order.iter().position(|id| *id == s.id).unwrap_or(usize::MAX));
        }
        list
    }

    // --- Acciones ---

    /// La página se abrió: si falta la lista o algún estado, se consulta
    /// (el núcleo guarda 30 s cada ping).
    pub fn shown(&self) {
        let missing = match self.shell.servers() {
            None => true,
            Some(l) => l.iter().any(|s| self.shell.status(&s.address).is_none()),
        };
        if missing && !self.pinging.replace(true) {
            self.actions.reload();
            self.refresh();
        }
    }

    /// «Actualizar estado»: olvida los estados y vuelve a consultar.
    pub fn refresh_status(&self) {
        if self.pinging.replace(true) {
            return;
        }
        self.shell.clear_statuses();
        self.actions.reload();
        self.refresh();
    }

    /// La UI ya copió la dirección (TextInput oculto): solo el aviso.
    fn copy(&self, id: &str) {
        if self.server(id).is_some() {
            self.shell.toast(&i18n::text(&self.lang(), "copied"), false);
        }
    }

    fn remove(&self, id: &str) {
        if let Some(s) = self.server(id) {
            self.actions.remove(&s.id, &s.name);
        }
    }

    pub fn removed(&self, result: Result<(), Failure>) {
        match result {
            Ok(()) => self.actions.reload(),
            Err(f) => self.shell.toast(&f.message, true),
        }
    }

    fn import(&self, instance_id: &str) {
        let name = self.shell.model.borrow().by_id(instance_id).map(|r| r.name.clone());
        if let Some(name) = name {
            self.actions.import(instance_id, &name);
        }
    }

    pub fn imported(&self, instance_name: &str, result: Result<usize, Failure>) {
        let lang = self.lang();
        match result {
            Ok(0) => self.shell.toast(&i18n::text(&lang, "serversImportNone").replace("{instance}", instance_name), false),
            Ok(n) => {
                let text = crate::content::message(&lang, "serversImported", &[("count", &n.to_string()), ("instance", instance_name)]);
                self.shell.toast(&text, false);
                self.actions.reload();
            }
            Err(f) => self.shell.toast(&f.message, true),
        }
    }

    /// Arrastre soltado: se ve ya en su sitio y se guarda.
    pub fn move_to(&self, id: &str, index: usize) {
        let mut ids: Vec<String> = self.list().into_iter().map(|s| s.id).collect();
        let Some(from) = ids.iter().position(|x| x == id) else { return };
        let index = index.min(ids.len() - 1);
        if from == index {
            return;
        }
        let s = ids.remove(from);
        ids.insert(index, s);
        *self.order.borrow_mut() = Some(ids);
        self.actions.move_to(id, index);
        self.refresh();
    }

    pub fn moved(&self, result: Result<Vec<Server>, Failure>) {
        *self.order.borrow_mut() = None;
        match result {
            Ok(list) => self.shell.set_servers(list),
            Err(f) => {
                self.shell.toast(&f.message, true);
                self.actions.reload();
            }
        }
        self.refresh();
    }

    // --- Diálogo ---

    /// Añadir (`None`) o editar.
    pub fn open_dialog(&self, id: Option<String>) {
        let server = id.as_deref().and_then(|id| self.server(id));
        if id.is_some() && server.is_none() {
            return;
        }
        let (instance, account) = match &server {
            Some(s) => (s.instance_id.clone(), s.account_id.clone()),
            None => (self.shell.home_instance_id(), None),
        };
        *self.dialog.borrow_mut() = Dialog { open: true, editing: server.as_ref().map(|s| s.id.clone()), instance, account, ..Default::default() };
        if let Some(ui) = self.ui.upgrade() {
            let st = ui.global::<ServersState>();
            st.set_dialog_address(server.as_ref().map(|s| s.address.clone()).unwrap_or_default().into());
            st.set_dialog_name(server.as_ref().map(|s| s.name.clone()).unwrap_or_default().into());
        }
        self.refresh();
    }

    fn close_dialog(&self) {
        let busy = self.dialog.borrow().busy;
        if !busy {
            self.dialog.borrow_mut().open = false;
            self.refresh();
        }
    }

    fn refresh_dialog_error(&self) {
        if self.dialog.borrow().touched {
            self.refresh();
        }
    }

    fn save(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let st = ui.global::<ServersState>();
        let address = st.get_dialog_address().trim().to_owned();
        let draft = {
            let mut d = self.dialog.borrow_mut();
            if d.busy {
                return;
            }
            d.touched = true;
            if !valid_address(&address) {
                drop(d);
                self.refresh();
                return;
            }
            d.busy = true;
            d.error = None;
            Draft {
                id: d.editing.clone(),
                name: st.get_dialog_name().trim().to_owned(),
                address,
                instance_id: d.instance.clone(),
                account_id: d.account.clone(),
            }
        };
        self.actions.save(&draft);
        self.refresh();
    }

    pub fn saved(&self, result: Result<(), Failure>) {
        {
            let mut d = self.dialog.borrow_mut();
            d.busy = false;
            match result {
                Ok(()) => d.open = false,
                Err(f) => d.error = Some(f),
            }
        }
        if !self.dialog.borrow().open {
            // `ref.invalidate(serverStatusProvider)`: lista y estados de nuevo.
            self.actions.reload();
        }
        self.refresh();
    }

    pub fn dialog_open(&self) -> bool {
        self.dialog.borrow().open
    }

    // --- Vista ---

    fn card(&self, lang: &str, s: &Server, st: Option<&Status>) -> ServerCard {
        let t = |k: &str| i18n::text(lang, k);
        let m = self.shell.model.borrow();
        let online = st.is_some_and(|s| s.online);
        let motd = match st {
            None if self.pinging.get() => t("serverPinging"),
            None => String::new(),
            Some(s) if s.online => s.motd.clone(),
            Some(_) => t("serverUnreachable"),
        };
        let instance = s.instance_id.as_deref().and_then(|id| m.by_id(id));
        let ls = instance.map(|r| m.launches.state(&r.id)).unwrap_or_default();
        let running = ls.phase == crate::instances::Phase::Running;
        let favicon = st.and_then(|st| st.favicon_png_base64.as_deref()).and_then(|b64| self.shell.favicon(b64));
        ServerCard {
            id: s.id.as_str().into(),
            name: s.name.as_str().into(),
            address: s.address.as_str().into(),
            motd: motd.into(),
            known: st.is_some(),
            online,
            version: st.filter(|s| s.online).map(|s| s.version_name.clone()).unwrap_or_default().into(),
            players: home::players(lang, st).into(),
            sample: st.map(|s| s.sample.join("\n")).unwrap_or_default().into(),
            level: home::ping_level(st),
            ping: if online { t("serverPing").replace("{ms}", &st.map(|s| s.latency_ms).unwrap_or(0).to_string()) } else { "—".into() }
                .into(),
            has_favicon: favicon.is_some(),
            icon: favicon.unwrap_or_else(|| self.shell.landscape(&s.address, 56.0 / 6.0, 56.0, 56.0)),
            has_instance: instance.is_some(),
            instance: instance.map(instance_badge).unwrap_or_default(),
            can_join: instance.is_some() && !ls.busy() && !running,
            busy: ls.busy(),
            running,
            join_tooltip: match instance {
                None => t("serverNoInstance"),
                Some(r) if running => t("serverAlreadyRunning").replace("{instance}", &r.name),
                Some(r) => t("serverJoinWith").replace("{instance}", &r.name),
            }
            .into(),
        }
    }

    /// Copia el estado a la UI.
    pub fn refresh(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let lang = self.lang();
        let t = |k: &str| i18n::text(&lang, k);
        let st = ui.global::<ServersState>();
        let loaded = self.shell.servers();
        let list = self.list();
        let statuses: Vec<Option<Status>> = list.iter().map(|s| self.shell.status(&s.address)).collect();
        let online = statuses.iter().flatten().filter(|s| s.online).count();
        // La consulta termina cuando todas las direcciones tienen estado.
        if self.pinging.get() && loaded.is_some() && statuses.iter().all(Option::is_some) {
            self.pinging.set(false);
        }
        st.set_loaded(loaded.is_some());
        st.set_subtitle(
            if list.is_empty() {
                t("serversSubtitle")
            } else {
                crate::content::message(&lang, "serversCount", &[("count", &list.len().to_string()), ("online", &online.to_string())])
            }
            .into(),
        );
        st.set_pinging(self.pinging.get());
        let cards: Vec<ServerCard> = list.iter().zip(&statuses).map(|(s, st)| self.card(&lang, s, st.as_ref())).collect();
        sync(&self.cards, cards);
        // Importar: las instancias, como en el menú de Flutter.
        let m = self.shell.model.borrow();
        let imports: Vec<ServerMenu> = m
            .all
            .iter()
            .map(|r| ServerMenu { value: r.id.as_str().into(), label: r.name.as_str().into(), instance: instance_badge(r) })
            .collect();
        st.set_has_instances(!imports.is_empty());
        st.set_import_items(ModelRc::from(Rc::new(VecModel::from(imports))));

        // Diálogo.
        let d = self.dialog.borrow();
        st.set_dialog_open(d.open);
        if !d.open {
            return;
        }
        st.set_dialog_editing(d.editing.is_some());
        st.set_dialog_busy(d.busy);
        let address = st.get_dialog_address();
        st.set_dialog_error(if d.touched && !valid_address(&address) { t("serverAddressInvalid") } else { String::new() }.into());
        let mut iopts = vec![plain_option("", &t("serverInstanceNone"))];
        iopts.extend(m.all.iter().map(|r| instance_option(&lang, r)));
        st.set_dialog_instance_options(ModelRc::from(Rc::new(VecModel::from(iopts))));
        st.set_dialog_instance(d.instance.clone().unwrap_or_default().into());
        let mut aopts = vec![plain_option("", &t("serverAccountActive"))];
        aopts.extend(self.accounts.accounts().iter().map(|a| crate::SelectOption {
            value: a.id.as_str().into(),
            label: a.name.as_str().into(),
            subtitle: t(if a.microsoft { "accountMicrosoft" } else { "accountOffline" }).into(),
            lead_image: self.accounts.face(a),
            ..Default::default()
        }));
        st.set_dialog_account_options(ModelRc::from(Rc::new(VecModel::from(aopts))));
        st.set_dialog_account(d.account.clone().unwrap_or_default().into());
        st.set_dialog_has_error(d.error.is_some());
        if let Some(e) = &d.error {
            st.set_dialog_error_message(e.message.as_str().into());
            st.set_dialog_error_detail(e.detail.as_str().into());
        }
    }
}

/// Copia `new` en `model` tocando solo las filas que cambian.
fn sync<T: Clone + PartialEq + 'static>(model: &VecModel<T>, new: Vec<T>) {
    use slint::Model as _;
    if model.row_count() != new.len() {
        model.set_vec(new);
        return;
    }
    for (i, row) in new.into_iter().enumerate() {
        if model.row_data(i).as_ref() != Some(&row) {
            model.set_row_data(i, row);
        }
    }
}

/// [`ServerActions`] contra el núcleo.
pub struct CoreServers {
    pub backend: crate::backend::Backend,
    pub lang: String,
}

impl ServerActions for CoreServers {
    fn save(&self, d: &Draft) {
        let (b, lang, d) = (self.backend.clone(), self.lang.clone(), d.clone());
        self.backend.rt.spawn(async move {
            let r = match &d.id {
                None => b
                    .core
                    .add_server(lcore::servers::NewServer {
                        name: d.name.clone(),
                        address: d.address.clone(),
                        instance_id: d.instance_id.clone(),
                        account_id: d.account_id.clone(),
                    })
                    .await
                    .map(|_| ()),
                Some(id) => b
                    .core
                    .update_server(
                        id,
                        lcore::servers::ServerPatch {
                            name: Some(d.name.clone()),
                            address: Some(d.address.clone()),
                            instance_id: Some(d.instance_id.clone()),
                            account_id: Some(d.account_id.clone()),
                        },
                    )
                    .await
                    .map(|_| ()),
            };
            let r = r.map_err(|e| Failure::new(&lang, &e));
            post(move |s| s.saved(r));
        });
    }

    fn remove(&self, id: &str, _name: &str) {
        let (core, lang, id) = (self.backend.core.clone(), self.lang.clone(), id.to_owned());
        self.backend.rt.spawn(async move {
            let r = core.remove_server(&id).await.map(|_| ()).map_err(|e| Failure::new(&lang, &e));
            post(move |s| s.removed(r));
        });
    }

    fn move_to(&self, id: &str, index: usize) {
        let (core, lang, id) = (self.backend.core.clone(), self.lang.clone(), id.to_owned());
        self.backend.rt.spawn(async move {
            let r = core
                .move_server(&id, index)
                .await
                .map(|l| l.into_iter().map(Server::from).collect())
                .map_err(|e| Failure::new(&lang, &e));
            post(move |s| s.moved(r));
        });
    }

    fn import(&self, instance_id: &str, instance_name: &str) {
        let (core, lang, id, name) = (self.backend.core.clone(), self.lang.clone(), instance_id.to_owned(), instance_name.to_owned());
        self.backend.rt.spawn(async move {
            let r = core.import_servers_dat(&id).await.map_err(|e| Failure::new(&lang, &e));
            post(move |s| s.imported(&name, r));
        });
    }

    fn reload(&self) {
        self.backend.load_servers();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direcciones_validas() {
        assert!(valid_address("mc.hypixel.net"));
        assert!(valid_address(" play.example.net:25565 "));
        assert!(valid_address("[::1]:25565"));
        assert!(valid_address("localhost"));
        assert!(!valid_address(""));
        assert!(!valid_address("   "));
        assert!(!valid_address("mi servidor"));
        assert!(!valid_address("http://x.net/"));
        assert!(!valid_address("ñandú.net"));
    }
}
