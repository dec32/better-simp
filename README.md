# 简化字批评

该项目收录了对现有简化字方案的批评与订正，并提供根据表格自动生成 OpenCC 映射字典的工具。


## 基础

文件 `简化字批评.xlsx` 的诸工作表的基本格式如：

|繁体|简体|订正|兼容|
|:--|:--|:--|:--|
|寶|宝|||
|標|标|標||
|齣|出|齣？||
|懺|忏|⿰忄⿹𢦏业|懺|


「繁体」和「简体」两栏记录了《简化字总表》所规定的简化方案。「订正」栏供用户填写，留空表示认同该简化规则，填写则表示对该简化规则进行订正（复原或提供新字）。

允许用 IDS（表意文字描述序列）或任意其他的方式来描述未被编码的订正字，但此时需要在「兼容」栏提供一个替代字符供程序读取。在订正字后加中文问号 `？` 可令程序无视该订正。

若想新增不存在于《简化字总表》的简化规则，可使用「其他」工作表。


## 类推

工作表「类推」用于记录类推规则，格式如：

|繁体|简体|类推|增补|
|:--|:--|:--|:--|
|訁|讠|計计 訂订 訃讣 譏讥 識识||
|戠|只|識识 幟帜 織织 熾炽 職职|識䛊|

「繁体」和「简体」用于记录类推成立的前提，「类推」栏记录《简化字总表》第三表所收录的类推字，「增补」栏供用户增添类推字。若一则类推拥有多个前提，那么它应被**所有**前提的对应行所记录。如上表的「識识」就同时见于「訁讠」和「戠只」两行。

当某一类推规则的前提成立时，「类推」和「增补」两栏的简化规则将被输出到字典中。若一个繁体存在多个可用的类推简化字，程序会选择更符合用户偏好的那一个。如上表中，「識」存在「识」和「䛊」两种简化选择，若「表二」有内容：

|繁体|简体|订正|
|:--|:--|:--|
|訁|讠|訁|
|戠|只||

那么「䛊」将被采纳，因为「识」动用了「訁讠」这一被用户否定的简化规则，而「䛊」则和用户的偏好完美契合。

各工作表的「简体」栏均不含类推相关的信息，与原始的《简化字总表》并不完全相同，比如「纖纤」一项就记为了「纖䊹」，「纟」旁的简化由工作表「类推」的「糹纟」行的「䊹纤」一项来控制。

若对自动类推的处理结果不满，可在工作表「其他」手动类推。「其他」的优先级是最高的，会覆盖自动类推的结果。

## 方案生成

确保文件 `简化字批评.xlsx` 与程序 `simp.exe` 在同一目录下，双击运行，即可在同一目录下得到映射字典 `TSCharacters.txt`。

或使用命令来指定表格和字典的路径：

```
simp [--input <表格路径>][--output <字典路径>][--rime]
```

## 集成到 RIME

执行命令 `simp --rime`，该命令会：

1. 把映射字典写入到 `%APPDATA%/Rime/opencc/TPCharacters.txt`
2. 在同一目录下新建相应的 OpenCC 配置 `t2p.json`

之后为输入法配置功能开关以及简化器。以「朙月拼音」为例，在 `%APPDATA%/Rime` 下新建补丁文件 `luna_pinyin.schema.custom.yaml`：

```YAML
patch:
  switches:
    - name: ascii_mode
      reset: 0
      states: [ 中文, 西文 ]
    - name: full_shape
      states: [ 半角, 全角 ]
    - options: [ trad, priv, simp ] # 用来控制简化器的三个互斥选项，取代了 `name: simplification`
      states: [ 繁體, 䋣躰, 简体 ]   # 选项的名字
    - name: ascii_punct
      states: [ 。，, ．， ]
  engine/filters:
    - simplifier@simp # 标准简化器
    - simplifier@priv # 自定简化器
    - uniquifier
  simp:
    option_name: simp        # 当 switches 中的选项 simp 被激活时
    opencc_config: t2s.json  # 使用标准简化配置 t2s.json
  priv:
    option_name: priv        # 当 switches 中的选项 priv 被激活时
    opencc_config: t2p.json  # 使用自定简化配置 t2p.json
```

