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

fn dirs(dir,ret,playlist) {
    let sl = ":" + playlist + ":";
    for entry in open_dir(dir) {
        if entry.is_dir {
            let ea = (""+entry).split("/");
            let sd = ":" + ea[ea.len-1] + ":";
            print("Check dir: " + sd);
            if sl=="::" || sl.contains(sd) {
                ret.push(entry)
            }
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

fn dir_name(path) {
    let a = (""+path).split("/");
    return a[a.len-1];
}

fn title_from_file(path) {
    let a = path.split("/");
    let fname = a[a.len-1];
    let l = fname.len;
    if fname[l-4] == '.' { return fname[0..l-4] }
    else { return fname[0..l-3] }
}

fn rnd_file(lib,playlist) {
    print("Playlist [" + playlist + "]");
    let sz_lim = 1024*1024*2;
    let base = "/home/ppc/" + lib;
    let v_dirs = dirs(base,[],playlist);
    let all = [];
    do {
        let di = rng_rand(v_dirs.len);
        all = all_files(v_dirs[di],[],sz_lim);
    } while all.len == 0;

    let ai = rng_rand(all.len);
    return all[ai].to_string();
}

fn shows(eve,lib) {
    let base = "/home/ppc/" + lib;
    let di = dirs(base,[],"");
    let cmp = "";
    for d in di {
        cmp += ":" + dir_name(d);
    }
    eve = result(eve, "_", cmp[1..]);
    return eve;
}

fn next(eve,lib,mode,playlist) {
    let file = rnd_file(lib,playlist);
    eve = result(eve, "title.content", title_from_file(file));
    eve = result(eve, "_", file);
    return eve;
}

fn on_event(eve,id,name,x,y,button) {
      if name=="change" && id=="filter_selected" {
          eve = result(eve, "q*", "die");

          return eve;
      }
      if name!="b_up" { return eve; }

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
            "season-fit"=> {
                eve = result(eve, id + ".content", ">>");
            }
            "show-unwatched"=> {
                eve = result(eve, id + ".checked", "!");
                eve = result(eve, "qfull.unwatched", "!");
            }
            "show-lock" => {
                eve = result(eve, id + ".checked", "!");
                eve = result(eve, "qfull.lock", "!");
            }
            "more-bkg" => {
                eve = result(eve, "bkg.control", "die");
            }
            "act-filter" => {
                eve = result(eve, "filter.visible", "!");
            }
            "lib-music" => {
                eve = result(eve, "globals.curlib", "Music");
                eve = result(eve, "layers.players", "media-full");
            }
            "lib-tv" => {
                eve = result(eve, "globals.curlib", "Videos/TV");
                eve = result(eve, "layers.players", "media-full");
            }
            "lib-film" => {
                eve = result(eve, "globals.curlib", "Videos/Film");
                eve = result(eve, "layers.players", "media-full");
            }
            "lib-xxx" => {
                eve = result(eve, "globals.curlib", "Videos/XXX");
                eve = result(eve, "layers.players", "media-quad");
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