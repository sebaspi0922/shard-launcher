#!/usr/bin/env python3
"""Licencias de las dependencias de Shard (lcore y shard-ui, sin las de
desarrollo) frente a GPLv3.

  tools/licencias.py            resumen y exit 1 si alguna no está en la lista
  tools/licencias.py --todas    además, cada crate con su licencia

Una expresión SPDX vale si alguna de sus alternativas (OR) solo usa licencias
compatibles con GPLv3. Sin dependencias: lee `cargo metadata`.
"""
import collections, json, re, subprocess, sys

# Compatibles con GPLv3 (FSF: https://www.gnu.org/licenses/license-list.html).
OK = {
    "MIT", "MIT-0", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause",
    "ISC", "Zlib", "Unicode-3.0", "Unicode-DFS-2016", "MPL-2.0", "CC0-1.0", "0BSD", "Unlicense",
    "BSL-1.0", "GPL-3.0-only", "GPL-3.0-or-later", "LGPL-3.0-or-later", "LGPL-2.1-or-later",
    "CDLA-Permissive-2.0", "bzip2-1.0.6",
}


def compatible(expr: str) -> bool:
    """Evalúa la expresión SPDX: AND pesa más que OR; los paréntesis se
    respetan. Una expresión mal formada no vale."""
    tokens = re.findall(r"\(|\)|[^\s()]+", expr.replace("/", " OR "))
    pos = 0

    class Mal(Exception):
        pass

    # idstring, o una referencia de otro documento (DocumentRef-x:LicenseRef-y).
    ident = re.compile(r"[A-Za-z0-9][A-Za-z0-9.+-]*")
    docref = re.compile(r"DocumentRef-[A-Za-z0-9.-]+:LicenseRef-[A-Za-z0-9.-]+")

    def id_(t: str, licencia: bool = True) -> str:
        ok = ident.fullmatch(t) or (licencia and docref.fullmatch(t))
        if not ok or t in ("AND", "OR", "WITH"):
            raise Mal
        return t

    def take():
        nonlocal pos
        if pos >= len(tokens):
            raise Mal
        pos += 1
        return tokens[pos - 1]

    def atom() -> bool:
        t = take()
        if t == "(":
            v = any_of()
            if take() != ")":
                raise Mal
            return v
        id_(t)
        if pos < len(tokens) and tokens[pos] == "WITH":
            take()
            exc = id_(take(), licencia=False)
            return f"{t} WITH {exc}" in OK
        return t in OK

    def all_of() -> bool:
        v = atom()
        while pos < len(tokens) and tokens[pos] == "AND":
            take()
            v = atom() and v
        return v

    def any_of() -> bool:
        v = all_of()
        while pos < len(tokens) and tokens[pos] == "OR":
            take()
            v = all_of() or v
        return v

    try:
        v = any_of()
    except Mal:
        return False
    return v and pos == len(tokens)


# Casos que no pueden romperse (se comprueban en cada ejecución).
assert compatible("MIT OR Apache-2.0")
assert compatible("(MIT OR Apache-2.0) AND Unicode-3.0")
assert compatible("Apache-2.0 WITH LLVM-exception OR MIT")
assert not compatible("(MIT OR Apache-2.0) AND LicenseRef-Incompatible")
assert not compatible("LicenseRef-Slint-Royalty-free-2.0")
assert not compatible("MIT ) AND LicenseRef-Incompatible")
assert not compatible("Apache-2.0 WITH")
assert not compatible("(MIT OR Apache-2.0")
assert not compatible("MIT OR Apache-2.0 WITH ???")
assert compatible("MIT OR DocumentRef-external:LicenseRef-custom")
assert not compatible("MIT OR DocumentRef-external:Apache-2.0")
assert not compatible("MIT WITH DocumentRef-external:LicenseRef-custom")


def main() -> int:
    meta = json.loads(subprocess.check_output(["cargo", "metadata", "--format-version", "1", "--locked"]))
    pkgs = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    todo = [p["id"] for p in meta["packages"] if p["name"] in ("lcore", "shard-ui")]
    seen = set()
    while todo:
        i = todo.pop()
        if i in seen:
            continue
        seen.add(i)
        for d in nodes[i]["deps"]:
            if any(k["kind"] != "dev" for k in d["dep_kinds"]):
                todo.append(d["pkg"])
    count = collections.Counter()
    bad = []
    for i in sorted(seen, key=lambda i: pkgs[i]["name"]):
        p = pkgs[i]
        lic = p.get("license") or f"(archivo: {p.get('license_file')})"
        count[lic] += 1
        if not compatible(lic):
            bad.append((p["name"], p["version"], lic))
        if "--todas" in sys.argv:
            print(f"{p['name']} {p['version']}: {lic}")
    print(f"{len(seen)} crates")
    for lic, n in count.most_common():
        print(f"{n:4} {lic}")
    for name, ver, lic in bad:
        print(f"REVISAR {name} {ver}: {lic}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
