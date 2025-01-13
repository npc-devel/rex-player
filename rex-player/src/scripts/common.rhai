
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
    return all[ai].to_string();
}