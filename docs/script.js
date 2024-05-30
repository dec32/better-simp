function toggle(filter) {
    let filters = document.querySelector(".filters").querySelectorAll(".filter")
    let rows = document.querySelectorAll(".row")
    let on = []
    let off = []
    for (let filter of filters) {
        if (filter.classList.contains("disabled")) {
            off.push(filter)
        } else {
            on.push(filter)
        }
    }
    
    let reset = false;
    if (off.length == 0) {
        // 在全选状态点击标签时，单选此标签
        for (let f of filters) {
            if (f == filter) {
                continue
            }
            f.classList.add("disabled")
        }
    } else if (off.length == 1){
        // 选中最后一个标签，自然回到全选状态
        filter.classList.remove("disabled");
        reset = true
    }else if (on.length == 1 && on[0] == filter) {
        // 取消最后一个标签，强行跳回全选状态
        for (let filter of filters) {
            filter.classList.remove("disabled")
        }
        reset = true
    }  else if(filter.classList.contains("disabled")) {
        // 普通的 flip 逻辑
        filter.classList.remove("disabled")
    } else {
        // 普通的 flip 逻辑
        filter.classList.add("disabled")
    }
    

    if (reset) {
        // 注意：全选时要把无标签的行也显示出来
        for (let row of rows) {
            row.style.display = "flex"
        }
    } else {
        let tags = []
        for (let f of filters) {
            if (f.classList.contains("disabled")) {
                continue
            }
            tags.push(f.innerHTML.substring(0, f.innerHTML.indexOf("(")))
        }

        for (let row of rows) {
            row.style.display = "none"
            for (let tag of row.querySelectorAll(".tag")) {
                tag = tag.innerHTML
                if (tags.includes(tag)) {
                    row.style.display = "flex"
                    break
                }
            }
        }
    }
}