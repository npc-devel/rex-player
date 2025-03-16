fn rng_rand(lim) {
    let rnd = rand();
    if rnd < 0 { rnd *= -1 }
    return rnd % lim;
}

fn result_init(lib,base) {
    let ret = #{
        "_":"?",
        "base_path": base,
        "cur_lib": lib,
        "tablet-mode": "n"
    };
    return ret;
}

fn result(ret,key,value) {
    ret[key] = value;
    return ret;
}

fn result_complete(ret) {
    let rs = "";
    for k in ret.keys() {
        if k!="_" && !k.contains(".") { continue }
        //print("Script result: " + k + " = " + ret[k]);
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

fn open_dir_sorted(dir) {
    let paths = [];
    for entry in open_dir(dir) {
        if entry.is_dir {
            paths.push(":" + entry);
        } else {
            paths.push("" + entry);
        }
    }
    paths.sort();
    paths
}

fn all_files(dir,ret,sz_lim) {
    let dirs = [];
    for entry in open_dir_sorted(dir) {
        if entry[0]==":" {
            ret = all_files(entry[1..],ret,sz_lim);
        } else {
            if bad_ext(entry) { continue }

            let file = open_file(entry);
            if file.bytes() < sz_lim  { continue }
            ret.push(entry);
        }
    }
    ret.sort();
    return ret;
}

fn dirs(base,dir,ret,playlist) {
    let sl = ":" + playlist + ":";
    for entry in open_dir(dir) {
        if !entry.is_dir { continue }
        if sl=="::" { ret.push(entry.to_string()); continue }

        let rel = (""+entry)[base.len..];
        let ea = rel.split("/");
        let sd = ":" + ea[1] + ":";
        let sd2 = sd;
        if ea.len > 2 {
            sd2 = ":" + ea[2] + ":";;
        }
        if sl.contains(sd) || sl.contains(sd2)   {
            ret.push(entry.to_string())
        }
    }
    ret.sort();
    return ret;
}

fn files(dir,ret,sz_lim,playlist) {
    let sl = ":" + playlist + ":";
    for entry in open_dir(dir) {
        if !entry.is_dir {
            if bad_ext(entry) { continue }
            let file = open_file(entry);
            if file.bytes() < sz_lim  { continue }
            let ep = ":" + entry_name(entry) + ":";
            if sl=="::"||sl.contains(ep) { ret.push(entry) }
        }
    }
    return ret;
}

fn update_ele(l,e,c,v) {
    return "->layer:" + l + ":elem:" + e + ":" + c + ":" + v + "\n";
}

fn entry_name(path) {
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

fn rnd_file(lib,eve) {
    let play_idx = eve["play_idx"].to_string();
    if play_idx==()||play_idx=="" { play_idx="-1"; }
    play_idx = parse_int(play_idx);

    if (""+eve["next_idx"])!="" {
        play_idx = parse_int(eve["next_idx"]);
    }

    let playlist = eve["playlist"];
    let is_unwatched = ""+eve["is_unwatched"];

    if (""+eve["next_show"])!="" {
        playlist = eve["next_show"];
    }

    print("Rnd file: [" + playlist + "] " + play_idx);
    let sz_lim = 1024*1024*2;
    let path = eve["base_path"] + lib;
    let v_dirs = dirs(eve["base_path"],path,["."],playlist);
    let all = [];
    do {
        let di = rng_rand(v_dirs.len);
        let cp = v_dirs[di];

        if cp == "." { all = files(path,[],sz_lim,playlist); }
        else { all = all_files(cp,[],sz_lim); }
    } while all.len == 0;
    all.sort();
    let l = all.len;
    if play_idx < 0 || play_idx>=l {
        let ai = rng_rand(all.len);
        eve = result(eve,"globals.file_idx",ai);
        eve = result(eve,"_",all[ai].to_string());
        return eve;
    } else {
        eve = result(eve,"globals.file_idx",play_idx);
        eve = result(eve,"_",all[play_idx].to_string());
        return eve;
    }
}

fn shows(eve,lib) {
    let path = eve["base_path"] + lib;
    let di = dirs(eve["base_path"],path,[],"");
    di.sort();
    let cmp = "";
    for d in di {
        cmp += ":" + entry_name(d);
    }
    let fi = files(path,[],2*1024*1024,"");
    fi.sort();
    for f in fi {
        cmp += ":" + entry_name(f);
    }
    eve = result(eve, "_", cmp[1..]);
    return eve;
}

fn show_from_file(s,eve,lib) {
    let path = eve["base_path"] + lib;
    let ts = s[path.len+1..].split("/");
    print("Show: " + ts[0]);
    return ts[0];
}

fn next(eve,lib,force_playlist) {
    if force_playlist!="" { eve["playlist"]=force_playlist }

    eve = rnd_file(lib,eve);
    eve = result(eve, "title.content", title_from_file(eve["_"]));
    eve = result(eve, "globals.cur_show", show_from_file(eve["_"],eve,lib));
    return eve;
}

fn on_event(eve,id,name,x,y,button) {
   //   print("Event: " + name);

      if name=="loaded" {
          eve = result(eve, "filter.title", eve["cur_lib"]);
          return eve;
      }

      if name=="change" {
          switch(id) {
              "globals.cur_show"|"globals.file_idx"|"globals.play_idx"=> {
                 let n = id[8..];
                // print("Change: " + n + " = [" + eve[n] + "]");
                 let sh = eve["cur_show"].to_string();
                 if sh == "" { return eve }

                 if sh == ":next:" {
                    eve = result(eve, "show-unwatched.checked", "false");
                    eve = result(eve, "globals.play_idx", -1);
                    eve = result(eve, "qfull.control", "die");
                //    print("Nextbail");
                    return eve;
                 }

                 let iw = setting_get("is_unwatched",sh,"false");
                 eve = result(eve, "show-unwatched.checked", iw=="true");
                 if iw=="false" {
                 //   print("Notwatched");
                    return eve
                 }

                 let idx = setting_get("play_idx",sh,"0");
                 if (eve["next_idx"]==idx || eve["file_idx"]==idx) && eve["cur_show"] == sh {
                     if eve["next_idx"]!=() && eve["next_idx"]!="" && eve["next_idx"]!=eve["file_idx"] {
                        setting_set("play_idx",eve["next_idx"],sh);
                     }
                     eve = result(eve, "globals.next_show", "");
                     eve = result(eve, "globals.next_idx", "");
                   //  print("All good");
                     return eve
                 }

                 eve = result(eve, "qfull.control", "die");
                 eve = result(eve, "globals.play_idx", idx);
                 eve = result(eve, "globals.next_show", sh);
               //  print("Req: " + sh + "@" + idx);
              }
              "filter.visible"=> {
                  if eve["filter.visible"]=="true" { return eve }
              }
              "filter.selected"=> {
                  print("New selected: " + eve["filter_selected"]);
                  eve = result(eve, "globals.playlist", eve["filter_selected"]);
              }
              "globals.playlist"=> {
                  let v = "";
                  v = v + eve["is_locked"];
                  if eve["playlist"]=="" || eve["playlist"]==eve["cur_show"] { return eve }

                 // print("Loading new playlist: " + eve["playlist"]);
                  eve = result(eve, "*.control", "die");
              }
              "globals.is_locked"=> {
                   let v = "";
                   v = v + eve["is_locked"];

                   if v!="true" {
                       print("Unlock playlist");
                       eve = result(eve, "globals.playlist", "");
                       return eve;
                   }
                   if eve["cur_show"]!=() && eve["cur_show"]!="" {
                   //    print("Lock playlist: [" + eve["cur_show"] + "]");
                       eve = result(eve, "globals.playlist", eve["cur_show"]);
                   }
              }
               _=> {}
          }

          return eve;
      }

      if id[0]=='q' {
            if eve["tablet-mode"]=="y" {
                switch name {
                  "b-long" =>{
                      //if id!="qfull" { eve = result(eve, id + ".control", "die") }
                      //eve = result(eve, "globals.cur_show", ":next:");
                  }
                  "b-dbl" => {
                      switch button {
                          6 => {
                              eve = result(eve, id + ".seek-rel", "-30");
                          }
                          7 => {
                              eve = result(eve, id + ".seek-rel", "30");
                          }
                          3 => {
                              if id!="qfull" { eve = result(eve, id + ".control", "die") }
                              eve = result(eve, "globals.cur_show", ":next:");
                          }
                          2 => {
                              eve = result(eve, id + ".clone", "q4");
                          }
                          1 => {
                              eve = result(eve, id + ".seek-abs", "-95");
                          }
                          _ => {}
                      }
                  }
                  "b-up" => {
                      //if button==1 {
                        //  eve = result(eve, id + ".seek-abs", "-95");
                      //} else if button==3 {
                        //  if id!="qfull" { eve = result(eve, id + ".control", "die") }
                         // eve = result(eve, "globals.cur_show", ":next:");
                     // }
                  }
                  _ => {}
              }
        } else {
            switch name {
                "scroll-up"=>{
                return result(eve, id + ".seek-rel", "40");
                }
                "scroll-down"=>{
                    return result(eve, id + ".seek-rel", "-60");
                }
                "b-long" =>{
                    if id!="qfull" { eve = result(eve, id + ".control", "die") }
                    eve = result(eve, "globals.cur_show", ":next:");
                }
                "b-dbl" => {
                    eve = result(eve, id + ".clone", "q4");
                }
                "b-up" => {
                    eve = result(eve, id + ".seek-abs", "-95");
                    eve = result(eve, id + ".seek-abs", "-90");
                }
                _ => {}
            }
        }
        return eve;
    } else {
        switch id {
            "next-show"=> {
                eve = result(eve, "qfull.control", "die");
                eve = result(eve, "globals.next_show", eve["cur_show"]);
                eve = result(eve, "globals.next_idx", parse_int(eve["file_idx"])+1);
            }
            "prev-show"=> {
                eve = result(eve, "qfull.control", "die");
                eve = result(eve, "globals.next_show", eve["cur_show"]);
                eve = result(eve, "globals.next_idx", parse_int(eve["file_idx"])-1);
            }
            "rnd-show"=> {
                eve = result(eve, "qfull.control", "die");
                eve = result(eve, "globals.next_show", eve["cur_show"]);
            }
            "aud-s"=> {
                eve = result(eve, "qfull.control", "next-audio");
            }
            "app-quit"=> {
                eve = result(eve, "app.control", "quit");
            }
            "season-fit"=> {
                eve = result(eve, id + ".content", ">>");
            }
            "show-unwatched"=> {
                let v = setting_get("is_unwatched",eve["cur_show"],"false");
                if v == "true" { v = "false" } else { v = "true" }

                setting_set("is_unwatched",v,eve["cur_show"]);
                eve = result(eve, "show-unwatched.checked", v=="true");
                if v=="true" {
                    eve = result(eve,"globals.file_idx","-1");
                }
            }
            "show-lock" => {
                let v = "";
                v = v + eve["is_locked"];
                eve = result(eve, id + ".checked", "!");
                eve = result(eve, "globals.is_locked", v!="true");
            }
            "more-bkg" => {
                eve = result(eve, "bkg.control", "die");
            }
            "act-filter" => {
                eve = result(eve, "filter.visible", "!");
            }
            "lib-music" => {
                eve = result(eve, "globals.next_show", "");
                eve = result(eve, "globals.cur_show", "");
                eve = result(eve, "globals.playlist", "");
                eve = result(eve, "globals.play_idx", "-1");
                eve = result(eve, "globals.curlib", "Music");
                eve = result(eve, "layers.players", "music");
            }
            "lib-tv" => {
                eve = result(eve, "globals.next_show", "");
                eve = result(eve, "globals.cur_show", "");
                eve = result(eve, "globals.playlist", "");
                eve = result(eve, "globals.play_idx", "-1");
                eve = result(eve, "globals.curlib", "Videos/TV");
                eve = result(eve, "layers.players", "media-full");
            }
            "lib-film" => {
                eve = result(eve, "globals.next_show", "");
                eve = result(eve, "globals.cur_show", "");
                eve = result(eve, "globals.playlist", "");
                eve = result(eve, "globals.play_idx", "-1");
                eve = result(eve, "globals.curlib", "Videos/Film");
                eve = result(eve, "layers.players", "media-full");
            }
            "lib-xxx" => {
                eve = result(eve, "globals.next_show", "");
                eve = result(eve, "globals.cur_show", "");
                eve = result(eve, "globals.playlist", "");
                eve = result(eve, "globals.play_idx", "-1");
                eve = result(eve, "globals.curlib", "Videos/XXX");
                eve = result(eve, "layers.players", "media-quad");
            }
            "sync1-p"|"sync1-m"=> {
                let sy = 0.0;
                if eve["sync1"]!=() && eve["sync1"]!="" { sy = parse_float(eve["sync1"]) }
                if id == "sync1-p" {
                    sy += 0.0005;
                } else {
                    sy -= 0.0005;
                }
                eve = result(eve, "globals.sync1", sy);
                eve = result(eve, "sync1.content", sy);
            }
            _ => {}
        }
    }

    return eve;
}

fn startup(allow_nsfw,base,dev) {
    let app = new_app(base);

    app.load_layer("players","media-full.view");
    if allow_nsfw {
        app.load_layer("overlay","osd.view");
    } else {
        app.load_layer("overlay","sfw-osd.view");
    }    
    app.load_layer("app","app-playlist.view");    
    if dev { app.load_layer("dev-overlay","dev-overlay.view"); }
    
    app.main_loop(1280,720);
}
