fn rng_rand(lim) {
    let rnd = rand();
    if rnd < 0 { rnd *= -1 }
    return rnd % lim;
}

fn result_init() {
    let ret = #{"_":"?"};
    return ret;
}

fn result(ret,key,value) {
    ret[key] = value;
    return ret;
}

fn result_complete(ret) {
    let rs = "";
    for k in ret.keys() {
        rs += k + "\n";
        rs += ret[k] + "\n";
    }
    return rs;
}

fn bad_ext(pbuf) {
    let bad = ["jpg","png","nfo","url","txt","sfv","srr","sub","subs"];
    let bstr = "" + pbuf;

    let el = bstr.len;
    let ext = bstr[el-3..el];

    for b in bad {
        if b == ext { return true }
        if b == ext.make_lower() { return true }
    }

    return false;
}

fn all_files(dir,ret,sz_lim) {
    for entry in open_dir(dir) {
        if entry.is_dir {
            ret = all_files(entry,ret,sz_lim);
        } else {
            if bad_ext(entry) { continue }

            let file = open_file(entry);
            if file.bytes() < sz_lim  { continue }
            ret.push(entry);
        }
    }
    return ret;
}

fn dirs(dir,ret) {
    for entry in open_dir(dir) {
        if entry.is_dir {
            ret.push(entry);
        }
    }
    return ret;
}

fn files(dir,ret,sz_lim) {
    for entry in open_dir(dir) {
        if !entry.is_dir {
            if bad_ext(entry) { continue }
            let file = open_file(entry);
            if file.bytes() < sz_lim  { continue }
            ret.push(entry);
        }
    }
    return ret;
}

fn update_ele(l,e,c,v) {
    return "->layer:" + l + ":elem:" + e + ":" + c + ":" + v + "\n";
}

fn file_name(path) {
    let a = path.split("/");
    return a[a.len-1];
}

fn rnd_file(lib) {
    let sz_lim = 1024*1024*2;
    let base = "/home/ppc/" + lib;
    let v_dirs = dirs(base,[]);
    let all = [];
    do {
        let di = rng_rand(v_dirs.len+1);
        if di < v_dirs.len { all = all_files(v_dirs[di],[],sz_lim); }
        else { all = files(base,[],sz_lim); }
    } while all.len == 0;

    let ai = rng_rand(all.len);
    return all[ai].to_string();
}

fn next(eve,lib,mode) {
    let file = rnd_file(lib);
    eve = result(eve, "_", file);
    return eve;
}

fn on_event(eve,id,name,x,y,button) {
      if id[0]=='q' {
        switch button {
            2 => {
                eve = result(eve, id + ".control", "die");
            }
            3 => {
                eve = result(eve, id + ".clone", "q4");
            }
            1 => {
                eve = result(eve, id + ".seek-abs", "-50");
            }
            4 => {
                eve = result(eve, id + ".seek-rel", "20");
            }
            5 => {
                eve = result(eve, id + ".seek-rel", "-20");
            }
            _ => {}
        }
    } else {
        switch id {
            "lib-music" => {
                eve = result(eve, "global.curlib", "Music");
                eve = result(eve, "layer.players", "media-full");
            }
            "lib-tv" => {
                eve = result(eve, "global.curlib", "Videos/TV");
                eve = result(eve, "layer.players", "media-full");
            }
            "lib-film" => {
                eve = result(eve, "global.curlib", "Videos/Film");
                eve = result(eve, "layer.players", "media-full");
            }
            "lib-xxx" => {
                eve = result(eve, "global.curlib", "Videos/XXX");
                eve = result(eve, "layer.players", "media-quad");
            }
            _ => {}
        }
    }

    return eve;
}

fn startup() {
    let app = new_app();

    app.load_layer("players","media-full.view");
    app.load_layer("app","app-playlist.view");
    app.load_layer("overlay","osd.view");

    app.main_loop(1280,720);
}