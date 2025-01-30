fn all_files(dir,ret) {
    for entry in open_dir(dir) {
        if entry.is_dir {
            ret = all_files(entry,ret);
        } else {
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

fn files(dir,ret) {
    for entry in open_dir(dir) {
        if !entry.is_dir {
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

fn next(lib,mode) {
    let base = "/home/ppc/Videos/" + lib;
    let v_dirs = dirs(base,[]);

    let all = [];
    do {
        let di = rand() % (v_dirs.len+1);
        if di < v_dirs.len { all = all_files(v_dirs[di],[]); }
        else { all = files(base,[]); }
    } while all.len == 0;

    let ai = rand() % all.len;
    const file = all[ai].to_string();
    if lib == "XXX" {
        return update_ele("overlay","#title","content","XXX") + file;
    } else {
        return update_ele("overlay","#title","content",file_name(file)) + file;
    }
}

fn startup() {
    let app = new_app();
    app.load_layer("players","media-quad.view");
    app.load_layer("overlay","osd.view");

    app.main_loop(1280,720);
}