import subprocess
import platform
import xml.etree.ElementTree as ET
from datetime import datetime
import sys
from pathlib import Path

scripts_dir = Path(sys.argv[0]).parent
repo_root = scripts_dir / ".."
wrap_dir = repo_root / "rust" / "crates" / "wrap"


def get_version(cmd):
    try:
        print(f"running {cmd}")
        result = subprocess.run(
            cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True
        )
        first_line = result.stdout.splitlines()[0]
        version = next(
            (word for word in first_line.split() if any(c.isdigit() for c in word)),
            "unknown",
        )
        return version
    except Exception:
        return "unknown"


def build_component(
    name, version, purl, license_ids, tool_type="application", vendor=None
):
    comp = ET.Element("component", {"type": tool_type})
    ET.SubElement(comp, "name").text = name
    ET.SubElement(comp, "version").text = version
    ET.SubElement(comp, "purl").text = purl
    if vendor:
        ET.SubElement(comp, "supplier").text = vendor
    licenses = ET.SubElement(comp, "licenses")
    for lic in license_ids:
        l = ET.SubElement(licenses, "license")
        ET.SubElement(l, "id").text = lic
    return comp


def main():
    OUTPUT = wrap_dir / "sbom-buildtools.cdx.xml"
    timestamp = datetime.utcnow().isoformat(timespec="seconds") + "Z"

    rustc_ver = get_version(["rustc", "--version"])
    cargo_ver = get_version(["cargo", "--version"])
    maturin_ver = get_version(["maturin", "--version"])
    uv_ver = get_version(["uv", "--version"])
    python_ver = platform.python_version()

    ns = "http://cyclonedx.org/schema/bom/1.5"
    ET.register_namespace("", ns)
    bom = ET.Element("bom", {"xmlns": ns, "version": "1"})
    metadata = ET.SubElement(bom, "metadata")
    ET.SubElement(metadata, "timestamp").text = timestamp
    tools = ET.SubElement(metadata, "tools")

    def add_tool(name, vendor, version):
        tool = ET.SubElement(tools, "tool")
        ET.SubElement(tool, "vendor").text = vendor
        ET.SubElement(tool, "name").text = name
        ET.SubElement(tool, "version").text = version

    add_tool("rustc", "Rust Project Developers", rustc_ver)
    add_tool("cargo", "Rust Project Developers", cargo_ver)
    add_tool("maturin", "PyO3 Developers", maturin_ver)
    add_tool("uv", "Astral", uv_ver)

    components = ET.SubElement(bom, "components")
    components.append(
        build_component(
            "rustc", rustc_ver, f"pkg:generic/rustc@{rustc_ver}", ["MIT", "Apache-2.0"]
        )
    )
    components.append(
        build_component(
            "cargo", cargo_ver, f"pkg:generic/cargo@{cargo_ver}", ["MIT", "Apache-2.0"]
        )
    )
    components.append(
        build_component(
            "maturin",
            maturin_ver,
            f"pkg:pypi/maturin@{maturin_ver}",
            ["MIT", "Apache-2.0"],
        )
    )
    components.append(
        build_component("uv", uv_ver, f"pkg:pypi/uv@{uv_ver}", ["MIT", "Apache-2.0"])
    )
    components.append(
        build_component(
            "CPython",
            python_ver,
            f"pkg:generic/python@{python_ver}",
            ["PSF-2.0"],
            tool_type="framework",
        )
    )

    tree = ET.ElementTree(bom)
    ET.indent(tree, space="  ", level=0)
    tree.write(OUTPUT, encoding="utf-8", xml_declaration=True)
    print(f"SBOM written to {OUTPUT}")


if __name__ == "__main__":
    main()
