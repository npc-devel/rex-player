
fn ret() {
    print("}");
}
fn cnt(p) {
    print(", cnt:'" + p + "'");
    return #{
        cnt: p,
        ret: ret };
}
fn cre(p) {
    print(",{ tag:'" + p + "'");
    return #{  tag: p,
                cnt: cnt };
}