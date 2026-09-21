use opencc_fmmseg::OpenCC;

const TRADITIONAL: &str = "你好，小篆國際編碼18";
const SIMPLIFIED: &str = "你好，小篆国际编码18";
const SEAL: &str = "你𿒛，𽌠𽴖𾇓𿭖𿛛碼18";

#[test]
fn t2seal() {
    let cc = OpenCC::from_dicts();
    assert_eq!(cc.t2seal(TRADITIONAL, false), SEAL);
}

#[test]
fn s2seal() {
    let cc = OpenCC::from_dicts();
    assert_eq!(cc.s2seal(SIMPLIFIED, false), SEAL);
}

#[test]
fn seal2t() {
    let cc = OpenCC::from_dicts();
    assert_eq!(cc.seal2t(SEAL, false), TRADITIONAL);
}

#[test]
fn seal2s() {
    let cc = OpenCC::from_dicts();
    assert_eq!(cc.seal2s(SEAL, false), SIMPLIFIED);
}

#[test]
fn inspect_seal_conversions() {
    let cc = OpenCC::from_dicts();

    let input = "你好，小篆國際編碼18";

    let t2seal = cc.t2seal(input, false);
    let s2seal = cc.s2seal(input, false);

    println!("input   : {input}");
    println!("t2seal  : {t2seal}");
    println!("s2seal  : {s2seal}");

    println!("seal2t  : {}", cc.seal2t(&t2seal, false));
    println!("seal2s  : {}", cc.seal2s(&s2seal, false));

    /*
    Output:
    input   : 你好，小篆國際編碼18
    t2seal  : 你𿒛，𽌠𽴖𾇓𿭖𿛛碼18
    s2seal  : 你𿒛，𽌠𽴖𾇓𿭖𿛛碼18
    seal2t  : 你好，小篆國際編碼18
    seal2s  : 你好，小篆国际编码18
    */
}

#[test]
fn inspect_seal_segmentation_boundary() {
    let cc = OpenCC::from_dicts();

    let simplified = "她的发色展示了他自己开发的新颖染发霜颜色";

    let seal = cc.s2seal(simplified, false);
    let back = cc.seal2s(&seal, false);

    println!("s2seal : {seal}");
    println!("seal2s : {back}");

    /*
    s2seal : 她𾌀𾧋𾨓𾡝𽀒𿯄他𽨋𿮛𿊵𿗺𾌀𿩱𾏷𿄻𾧋𿇝𾤽𾨓
    seal2s : 她的发色展示了他自己开发的新颖染发霜颜色
    */
}

#[test]
fn seal_round_trip_preserves_s2t_segmentation_boundaries() {
    let cc = OpenCC::from_dicts();

    const SIMPLIFIED: &str =
        "她的发色展示了他自己开发的新颖染发霜颜色";

    const SEAL: &str =
        "她𾌀𾧋𾨓𾡝𽀒𿯄他𽨋𿮛𿊵𿗺𾌀𿩱𾏷𿄻𾧋𿇝𾤽𾨓";

    assert_eq!(cc.s2seal(SIMPLIFIED, false), SEAL);
    assert_eq!(cc.seal2s(SEAL, false), SIMPLIFIED);
}