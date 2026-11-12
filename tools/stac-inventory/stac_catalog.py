"""Classifies the Stac model into inventory items (shared by generate.py and gen_rust.py)."""

import os
import re

import stac_model

# Batches (plan tasks) per widget directory (snake_case dir name under widgets/).
WIDGET_BATCHES = {
    "T10": """align aspect_ratio backdrop_filter center clip_oval clip_rrect colored_box column container
        divider expanded fitted_box flexible fractionally_sized_box icon image intrinsic_height intrinsic_width
        limited_box opacity padding placeholder positioned row safe_area selectable_text sized_box spacer stack
        text vertical_divider visibility wrap hero""",
    "T11": """carousel_view custom_scroll_view grid_view list_tile list_view page_view refresh_indicator
        single_child_scroll_view sliver_app_bar sliver_fill_remaining sliver_grid sliver_list sliver_opacity
        sliver_padding sliver_safe_area sliver_to_box_adapter sliver_visibility table table_cell table_row""",
    "T12": """auto_complete badge card check_box chip circle_avatar circular_progress_indicator dropdown_menu
        elevated_button filled_button floating_action_button form gesture_detector icon_button ink_well
        linear_progress_indicator outlined_button radio radio_group slider switch text_button text_field
        text_form_field tool_tip""",
    "T13": """alert_dialog app_bar bottom_navigation_bar bottom_navigation_view conditional
        default_bottom_navigation_controller default_navigation_controller default_tab_controller drawer
        dynamic_view navigation_bar navigation_view network_widget scaffold set_value tab tab_bar tab_bar_view""",
}
FOUNDATION_BATCHES = {
    "T6": "alignment geometry layout colors text specifications",
    "T7": "borders decoration effects animation interaction forms",
    "T8": "navigation ui_components theme",
}

CODEGEN_NOTE = "metadado de codegen Dart sem representacao no JSON"

# Dart-side helpers that have no JSON representation of their own.
UNSUPPORTED_NOTES = {
    "StacOptions": "configuracao do codegen Dart (CLI/projeto), sem representacao no JSON",
    "StacScreen": CODEGEN_NOTE,
    "StacThemeRef": CODEGEN_NOTE,
    "StacWidget": "base Dart (guarda JSON cru); representada pelo enum Widget",
    "StacAction": "base Dart (guarda JSON cru); representada pelo enum Action",
    "StacElement": "interface Dart de serializacao; sem representacao no JSON",
    "DoubleConverter": "conversor Dart de double; representado por f64 serializado como numero",
    "StacNavigator": "helpers estaticos Dart que constroem StacNavigateAction; sem representacao no JSON",
    "StacColors": "constantes de cor Dart; no JSON a cor e uma string (Color::hex / Color::new)",
    "StacThemeTextStyles": "atalhos Dart para StacThemeTextStyle; sem representacao no JSON",
    "StacThemeData": "helper estatico Dart para estilos de texto do tema; sem representacao no JSON",
}


def snake(name):
    s = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", name)
    s = re.sub(r"(?<=[A-Z])(?=[A-Z][a-z])", "_", s)
    return s.lower()


def dart_group(dart_path):
    parts = dart_path.split(os.sep)
    return parts


def batch_for(category, dart_path):
    parts = dart_path.split(os.sep)
    top = parts[0]
    if top == "widgets":
        d = parts[1]
        for b, names in WIDGET_BATCHES.items():
            if d in names.split():
                return b
        return "T10"
    if top == "actions":
        return "T9"
    if top == "foundation":
        g = parts[1]
        for b, names in FOUNDATION_BATCHES.items():
            if g in names.split():
                return b
        return "T8"
    if top == "core":
        return "T3"
    return "T8"  # annotations


def category_for(dart_path):
    top = dart_path.split(os.sep)[0]
    if top == "widgets":
        return "widget"
    if top == "actions":
        return "action"
    if top == "foundation":
        return "theme" if dart_path.split(os.sep)[1] == "theme" else "foundation"
    return "foundation"


def resolve_discriminator(cl, enums_by_name):
    """Returns the JSON discriminator value (string) for widgets/actions, else None."""
    g = cl.get("discriminator_getter")
    if g:
        expr = g["expr"]
        m = re.match(r"(?:WidgetType|ActionType)\.(\w+)\.name$", expr)
        if m:
            return m.group(1)
        m = re.match(r"'([^']*)'$", expr)
        if m:
            return m.group(1)
        raise ValueError("unknown discriminator expr %r in %s" % (expr, cl["name"]))
    st = cl.get("super_type")
    if st:
        return st["value"]
    return None


def build(lib_root):
    model = stac_model.load(lib_root)
    enums_by_name = {e["name"]: e for e in model["enums"]}
    items = []
    for e in model["enums"]:
        if e["name"] in ("WidgetType", "ActionType"):
            continue  # discriminator catalogs, covered by the widget/action items
        items.append({
            "name": e["name"], "kind": "enum", "category": category_for(e["dart_path"]),
            "dart_path": e["dart_path"], "json_type": None,
            "batch": batch_for(category_for(e["dart_path"]), e["dart_path"]),
        })
    for c in model["classes"]:
        cat = category_for(c["dart_path"])
        if c["name"] in ("StacWidget", "StacAction") or c["dart_path"].startswith(("core", "annotations")):
            cat = "foundation"
        if c["extends"] == "StacWidget" and c["discriminator_getter"]:
            cat = "widget"
        if c["extends"] == "StacAction" and c["discriminator_getter"]:
            cat = "action"
        item = {
            "name": c["name"], "kind": "class", "category": cat, "dart_path": c["dart_path"],
            "json_type": resolve_discriminator(c, enums_by_name) if (
                cat in ("widget", "action") or c.get("super_type")) else None,
            "batch": batch_for(cat, c["dart_path"]),
        }
        items.append(item)
    items.sort(key=lambda i: (i["category"], i["name"]))
    return model, items


def pascal(v):
    out = v[0].upper() + v[1:]
    if out[0].isdigit():
        out = "V" + out
    if out == "Self":
        out = "Self_"
    return out
