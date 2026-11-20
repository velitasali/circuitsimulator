use std::{env, fs};

use cs_engine::canvas::{Canvas, Part, Point};
use cs_engine::library::{Library, TYPE_CATEG_MAIN, TYPE_COMPONENT};
use cs_engine::{catalog, subcircuit};

const LED_PKG_CIRC1: &str = r##"<circuit version="1.0.0" rev="0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" animate="1" anicurr="1" ansi="0" width="3200" height="2400" >
<item itemtype="Led" CircId="Led-1" Color="Yellow" Grounded="false" Threshold="2.4 V" MaxCurrent="30 mA" Resistance="600 mΩ" Pos="-124,-36" ShowProp="Color" mainComp="true" />
<item itemtype="Tunnel" CircId="Tunnel-2" Name="anode" IsBus="false" Pos="-156,-36" mainComp="true" />
<item itemtype="Tunnel" CircId="Tunnel-3" Name="cathode" IsBus="false" Pos="-108,-52" mainComp="true" />
<item itemtype="SubPackage" CircId="SubPackage-2" Name="LS" Width="6" Height="5" SubcType="Logic" LogicSymbol="true" CustomColor="false" BckGndColor="#ebf0ff" Border="true" Background="" PackageFile="LS_LS.package" Pins="Pin; type=; xpos=-8; ypos=8; angle=180; length=8; space=0; id=anode; label=an&#xa;Pin; type=; xpos=56; ypos=8; angle=0; length=8; space=0; id=cathode; label=ca&#xa;" Pos="-116,12" ShowProp="SubcType" />
<item itemtype="Connector" CircId="Connector-1" startpinid="Tunnel-2-pin" endpinid="Led-1-lPin" pointList="-156,-36,-140,-36" />
<item itemtype="Connector" CircId="Connector-2" startpinid="Tunnel-3-pin" endpinid="Led-1-rPin" pointList="-108,-52,-100,-52,-100,-36,-108,-36" />
</circuit>"##;

#[test]
fn local_circ1_subcircuit_loads_in_library_and_canvas() {
    let root = env::temp_dir().join(format!("cs-local-e2e-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("LED_PKG")).unwrap();
    fs::write(root.join("LED_PKG").join("LED_PKG.circ1"), LED_PKG_CIRC1).unwrap();
    let root_circ = root.join("main.circ1");
    fs::write(&root_circ, r#"<circuit version="1.0.0" ></circuit>"#).unwrap();

    // 1. Library surfaces local subcircuits in an expanded Custom category at the top.
    let local = catalog::local_subcircuits(root_circ.to_str(), root.to_str());
    let mut lib = Library::new();
    lib.add_catalog_mcus(&local);
    let rows = lib.visible_rows();
    assert_eq!(rows[0].caption, catalog::LOCAL_SUBCIRCUIT_CATEGORY);
    assert_eq!(rows[0].item_type, TYPE_CATEG_MAIN);
    assert!(rows[0].expanded);
    assert!(rows[0].is_custom);
    assert_eq!(rows[1].caption, "LED_PKG");
    assert_eq!(rows[1].item_type, TYPE_COMPONENT);
    assert_eq!(rows[1].comp_type, "Subcircuit");
    assert!(rows[1].is_custom);

    // 2. Canvas places LED_PKG from an unsaved circuit using project_dir.
    let mut canvas = Canvas::empty();
    canvas.set_project_dir(root.to_str().map(str::to_string));
    canvas.add_component_at("LED_PKG,Subcircuit", Point::new(80.0, 80.0));
    let item = canvas
        .scene()
        .items()
        .iter()
        .find(|i| i.kind.type_name() == "Subcircuit")
        .expect("LED_PKG subcircuit placed via project_dir");
    let Part::Subcircuit(subc) = &item.kind else {
        panic!("expected Subcircuit");
    };
    assert_eq!(subc.device, "LED_PKG");
    assert!(
        subc.logic_symbol,
        "single LS SubPackage should keep logic_symbol=true"
    );
    assert_eq!(subc.package.pins.len(), 2);
    assert_eq!(subc.package.pins[0].id, "anode");
    assert_eq!(subc.package.pins[1].id, "cathode");
    let exposed = subcircuit::exposed_items(&subc.nested_src);
    assert!(exposed.iter().any(|e| e.circ_id == "Led-1"));

    // 3. Reloading a saved circuit in the project root restores LED_PKG cleanly.
    let saved = canvas.to_sim1();
    let mut canvas2 = Canvas::empty();
    canvas2
        .load_sim1(&saved, root_circ.to_str().map(str::to_string))
        .unwrap();
    let reloaded = canvas2
        .scene()
        .items()
        .iter()
        .find(|i| i.kind.type_name() == "Subcircuit")
        .expect("reloaded LED_PKG");
    let Part::Subcircuit(subc2) = &reloaded.kind else {
        panic!("expected Subcircuit");
    };
    assert_eq!(subc2.package.pins.len(), 2);
    assert!(subc2.logic_symbol);

    let _ = fs::remove_dir_all(&root);
}
