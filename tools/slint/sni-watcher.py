#!/usr/bin/env python3
"""StatusNotifierWatcher mínimo (lo que da un panel como waybar/KDE).

ksni (bandeja de Slint en Linux) exige un watcher para registrarse; en Bash no
hay panel. Este stub solo acepta los registros y expone las propiedades.
"""
from gi.repository import Gio, GLib

XML = """
<node><interface name="org.kde.StatusNotifierWatcher">
  <method name="RegisterStatusNotifierItem"><arg type="s" direction="in"/></method>
  <method name="RegisterStatusNotifierHost"><arg type="s" direction="in"/></method>
  <property name="RegisteredStatusNotifierItems" type="as" access="read"/>
  <property name="IsStatusNotifierHostRegistered" type="b" access="read"/>
  <property name="ProtocolVersion" type="i" access="read"/>
  <signal name="StatusNotifierItemRegistered"><arg type="s"/></signal>
  <signal name="StatusNotifierItemUnregistered"><arg type="s"/></signal>
  <signal name="StatusNotifierHostRegistered"/>
</interface></node>"""
items = []


def call(conn, sender, path, iface, method, params, inv):
    if method == "RegisterStatusNotifierItem":
        svc = params.unpack()[0]
        items.append(svc if svc.startswith(":") or "." in svc else sender)
        conn.emit_signal(None, path, iface, "StatusNotifierItemRegistered", GLib.Variant("(s)", (svc,)))
    inv.return_value(None)


def prop(conn, sender, path, iface, name):
    return {
        "RegisteredStatusNotifierItems": GLib.Variant("as", items),
        "IsStatusNotifierHostRegistered": GLib.Variant("b", True),
        "ProtocolVersion": GLib.Variant("i", 0),
    }[name]


def acquired(conn, name):
    node = Gio.DBusNodeInfo.new_for_xml(XML)
    conn.register_object("/StatusNotifierWatcher", node.interfaces[0], call, prop, None)


Gio.bus_own_name(Gio.BusType.SESSION, "org.kde.StatusNotifierWatcher", 0, acquired, None, None)
GLib.MainLoop().run()
