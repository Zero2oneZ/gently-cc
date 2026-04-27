//! CODIE glyph table — universal surface forms.
//!
//! Every primitive maps from ALL its surface forms across:
//!   - Programming languages: Python, JS/TS, Rust, Go, Java, C/C++, Ruby, Swift, Kotlin,
//!     Haskell, Lisp/Clojure, Lua, PHP, R, Julia, Scala, Elixir, Erlang
//!   - Natural languages: en, es, fr, de, pt, ru, zh, ja, ko, ar, hi
//!   - Operator symbols where conventional
//!
//! Rule: each surface form appears in exactly one arm. First match wins.
//! Ambiguous terms (e.g. "loop") are assigned to their dominant programming meaning.

/// Surface form → glyph. Case-insensitive (caller lowercases before calling).
pub fn to_glyph(kw: &str) -> Option<&'static str> {
    match kw {

        // ── ρ  pug: entry point / begin here ──────────────────────────────────
        "pug" | "main" | "entrypoint" | "entry_point" | "entry" | "init" | "run"
        | "programa" | "principal"                          // es: main
        | "début" | "commencer"                             // fr: begin
        | "anfang" | "einstieg"                             // de: start
        | "começar" | "início"                              // pt: begin
        | "начало" | "запуск"                               // ru: start/launch
        | "开始" | "主函数" | "入口"                         // zh: start / main
        | "시작" | "진입점"                                  // ko: start / entry
        | "الرئيسية" | "البداية"                            // ar: main / start
        | "शुरुआत" | "प्रारंभ"                             // hi: start
        => Some("ρ"),

        // ── β  bark: fetch / get / pull from source ───────────────────────────
        "bark" | "fetch" | "get" | "pull" | "load" | "retrieve" | "query"
        | "find" | "lookup" | "select" | "scan" | "read" | "recv" | "receive"
        | "import" | "require" | "include" | "use"
        | "obtener" | "buscar" | "recuperar"                // es
        | "récupérer" | "chercher" | "obtenir"              // fr
        | "abrufen" | "holen" | "laden"                     // de
        | "obter" | "buscar" | "carregar"                   // pt
        | "получить" | "найти" | "загрузить"               // ru
        | "获取" | "查询" | "加载" | "读取"                  // zh
        | "가져오기" | "조회" | "읽기"                       // ko
        | "الحصول" | "جلب"                                  // ar
        | "प्राप्त करना" | "लाना"                          // hi
        => Some("β"),

        // ── ς  spin: loop / repeat / iterate ──────────────────────────────────
        "spin" | "chase" | "loop" | "each" | "iter" | "iterate" | "foreach"
        | "repeat" | "times" | "cycle" | "walk" | "traverse" | "scan"
        | "reduce" | "collect" | "gather" | "accumulate"
        | "bucle" | "repetir" | "iterar" | "cada"           // es
        | "boucle" | "répéter" | "itérer" | "chaque"        // fr
        | "schleife" | "wiederholen" | "jedes"              // de
        | "laço" | "repetir" | "iterar" | "cada"            // pt
        | "цикл" | "повторить" | "перебрать"                // ru
        | "循环" | "遍历" | "重复" | "迭代"                  // zh
        | "반복" | "순환" | "각각"                           // ko
        | "حلقة" | "تكرار"                                  // ar
        | "लूप" | "दोहराना"                                 // hi
        => Some("ς"),

        // ── κ  cali: define function / here is how ────────────────────────────
        "cali" | "trick"
        | "def" | "fn" | "func" | "fun" | "function" | "method" | "procedure"
        | "proc" | "lambda" | "closure" | "defun" | "defn" | "sub" | "define"
        | "macro" | "decorator" | "op" | "operation" | "action" | "handler"
        | "funcion" | "función" | "método"                  // es
        | "fonction" | "méthode" | "procédure"              // fr
        | "funktion" | "methode" | "verfahren"              // de
        | "função" | "método" | "procedimento"              // pt
        | "функция" | "метод" | "процедура"                 // ru
        | "函数" | "方法" | "过程" | "定义"                  // zh
        | "함수" | "메서드" | "절차"                         // ko
        | "دالة" | "إجراء"                                  // ar
        | "फ़ंक्शन" | "विधि"                               // hi
        => Some("κ"),

        // ── ε  elf: bind variable / call this X ──────────────────────────────
        "elf" | "tag"
        | "let" | "var" | "val" | "mut" | "set" | "bind" | "assign" | "declare"
        | "local" | "global" | "name" | "label" | "alias"
        | "variable" | "nombre" | "identificador"           // es
        | "variable" | "nom" | "identifiant"                // fr
        | "variable" | "bezeichner" | "name"                // de
        | "variável" | "nome" | "identificador"             // pt
        | "переменная" | "имя" | "идентификатор"           // ru
        | "变量" | "名称" | "标识符"                         // zh
        | "변수" | "이름" | "식별자"                         // ko
        | "متغير" | "اسم"                                   // ar
        | "चर" | "नाम"                                      // hi
        => Some("ε"),

        // ── τ  turk: incomplete / needs work ──────────────────────────────────
        "turk" | "sniff"
        | "todo" | "fixme" | "hack" | "wip" | "stub" | "placeholder"
        | "incomplete" | "pending" | "missing" | "tbd" | "tbi" | "xxx" | "temp"
        | "draft" | "unfinished" | "broken" | "workaround"
        | "pendiente" | "incompleto" | "temporal"           // es
        | "en_cours" | "incomplet" | "temporaire"           // fr
        | "ausstehend" | "unvollständig" | "vorübergehend" // de
        | "pendente" | "incompleto" | "temporário"          // pt
        | "незавершено" | "временно" | "ожидает"           // ru
        | "待办" | "未完成" | "临时"                         // zh
        | "미완성" | "임시" | "보류"                         // ko
        => Some("τ"),

        // ── φ  fence: constraint / NOT this ───────────────────────────────────
        "fence"
        | "constraint" | "guard" | "require" | "ensure" | "enforce"
        | "precondition" | "invariant" | "restrict" | "forbid" | "deny"
        | "block" | "check" | "validate" | "verify" | "limit" | "bound"
        | "except" | "unless" | "without" | "excluding" | "neither"
        | "restricción" | "requisito" | "restricción"       // es
        | "contrainte" | "garde" | "restriction"            // fr
        | "einschränkung" | "bedingung" | "wächter"         // de
        | "restrição" | "requisito" | "guarda"              // pt
        | "ограничение" | "условие" | "проверка"           // ru
        | "约束" | "限制" | "条件" | "验证"                  // zh
        | "제약" | "조건" | "검증"                           // ko
        => Some("φ"),

        // ── π  pin: exact specification / precisely this ──────────────────────
        "pin" | "sit"
        | "exact" | "precise" | "specific" | "literal" | "fixed" | "concrete"
        | "definite" | "explicit" | "absolute" | "strict" | "verbatim"
        | "exacto" | "preciso" | "específico"               // es
        | "précis" | "exact" | "spécifique"                 // fr
        | "genau" | "präzise" | "spezifisch"                // de
        | "exato" | "preciso" | "específico"                // pt
        | "точно" | "конкретно" | "буквально"              // ru
        | "精确" | "确切" | "字面量"                         // zh
        | "정확" | "구체적" | "명시적"                       // ko
        => Some("π"),

        // ── Β  bone: immutable / cannot change ────────────────────────────────
        "bone"
        | "immutable" | "final" | "frozen" | "sealed" | "readonly" | "static"
        | "locked" | "permanent" | "constant" | "fixed" | "const"
        | "immovable" | "stable" | "invariant"
        | "inmutable" | "final" | "constante"               // es
        | "immuable" | "final" | "constant"                 // fr
        | "unveränderlich" | "endgültig" | "konstant"       // de
        | "imutável" | "final" | "constante"                // pt
        | "неизменный" | "постоянный" | "константа"        // ru
        | "不可变" | "最终" | "常量"                         // zh
        | "불변" | "최종" | "상수"                           // ko
        => Some("Β"),

        // ── Λ  blob: flexible / whatever works ────────────────────────────────
        "blob" | "play"
        | "any" | "dynamic" | "flexible" | "generic" | "abstract" | "variant"
        | "polymorphic" | "mixed" | "fluid" | "loose" | "optional" | "nullable"
        | "flexible" | "genérico" | "dinámico"              // es
        | "souple" | "générique" | "dynamique"              // fr
        | "flexibel" | "generisch" | "dynamisch"            // de
        | "flexível" | "genérico" | "dinâmico"              // pt
        | "гибкий" | "обобщённый" | "динамический"         // ru
        | "灵活" | "泛型" | "动态" | "可选"                  // zh
        | "유연" | "제네릭" | "동적"                         // ko
        => Some("Λ"),

        // ── μ  biz: goal / output / end state ─────────────────────────────────
        "biz" | "treat"
        | "goal" | "result" | "output" | "yield" | "produce" | "emit"
        | "render" | "respond" | "reply" | "answer" | "deliver" | "publish"
        | "send" | "post" | "put" | "push" | "write"
        | "objetivo" | "resultado" | "salida"               // es
        | "objectif" | "résultat" | "sortie"                // fr
        | "ziel" | "ergebnis" | "ausgabe"                   // de
        | "objetivo" | "resultado" | "saída"                // pt
        | "цель" | "результат" | "вывод"                   // ru
        | "目标" | "结果" | "输出" | "发送"                  // zh
        | "목표" | "결과" | "출력"                           // ko
        => Some("μ"),

        // ── ∆  anchor: save state / checkpoint ────────────────────────────────
        "anchor" | "bury"
        | "save" | "commit" | "checkpoint" | "persist" | "store" | "flush"
        | "snapshot" | "memorize" | "record" | "log" | "archive" | "backup"
        | "guardar" | "comprometer" | "almacenar"           // es
        | "sauvegarder" | "valider" | "stocker"             // fr
        | "speichern" | "sichern" | "archivieren"           // de
        | "salvar" | "confirmar" | "armazenar"              // pt
        | "сохранить" | "зафиксировать" | "записать"       // ru
        | "保存" | "提交" | "存储" | "记录"                  // zh
        | "저장" | "커밋" | "보관"                           // ko
        => Some("∆"),

        // ── Logic gates ────────────────────────────────────────────────────────
        "and" | "&&" | "band" | "bitand"
        | "et" | "und" | "y" | "e"
        | "и" | "та" | "且" | "그리고" | "و"               => Some("∧"),

        "or" | "||" | "bor" | "bitor"
        | "ou" | "oder" | "o" | "или" | "або"
        | "或" | "또는" | "أو"                              => Some("∨"),

        "not" | "!" | "~" | "bnot" | "bitnot" | "neg" | "negate"
        | "non" | "nicht" | "нет" | "не"
        | "否" | "아니" | "لا"                              => Some("¬"),

        "xor" | "^" | "eor" | "neq"                        => Some("⊕"),
        "nand"                                              => Some("⊼"),
        "nor"                                               => Some("⊽"),

        // ── Boolean ────────────────────────────────────────────────────────────
        "true"  | "wag"  | "yes" | "ok" | "on" | "enabled" | "success"
        | "oui" | "sí" | "si" | "sim" | "да" | "是" | "예" | "ja" | "نعم" | "हाँ"
        => Some("⊤"),

        "false" | "whine" | "no" | "off" | "disabled" | "failure" | "nil" | "null"
        | "none" | "void" | "never" | "nan" | "undefined"
        | "non" | "нет" | "否" | "아니요" | "nein" | "لا" | "नहीं"
        => Some("⊥"),

        // ── Control flow ───────────────────────────────────────────────────────
        // if
        "if" | "elif" | "elseif" | "unless" | "when" | "iff" | "given" | "provided"
        | "si" | "wenn" | "если" | "如果" | "만약" | "إذا" | "अगर"
        => Some("⁇"),

        // else
        "else" | "otherwise" | "default" | "fallback" | "fallthrough"
        | "sinon" | "sonst" | "sino" | "иначе" | "否则" | "그렇지않으면" | "وإلا" | "अन्यथा"
        => Some("∴"),

        // start/begin scope
        "start" | "begin" | "launch" | "open" | "activate" | "enable" | "boot"
        | "iniciar" | "empezar" | "démarrer" | "commencer" | "starten"
        | "começar" | "запустить" | "启动" | "시작하다" | "ابدأ"
        => Some("⊢"),

        // for loop
        "for" | "foreach" | "forin" | "forof" | "forall" | "forany"
        | "pour" | "para" | "für"
        | "для" | "为" | "위해" | "لأجل" | "के लिए"
        => Some("∀"),

        // fork/parallel
        "fork" | "spawn" | "parallel" | "concurrent" | "goroutine" | "thread"
        | "async" | "await" | "promise" | "future" | "task" | "coroutine" | "fiber"
        | "bifurcar" | "bifurquer" | "verzweigen"
        | "параллельно" | "并行" | "병렬"
        => Some("⋈"),

        // branch/switch/match
        "branch" | "switch" | "case" | "match" | "pattern" | "cond" | "dispatch"
        | "rama" | "branche" | "zweig" | "ветвь"
        | "分支" | "开关" | "분기"
        => Some("⊃"),

        // while loop (loop/do assigned here — most PLs use them as while-equivalents)
        "while" | "until" | "do" | "loop" | "solange" | "mientras" | "enquanto"
        | "tant que" | "tanque"
        | "пока" | "当" | "동안" | "بينما" | "जबकि"
        => Some("↺"),

        // break
        "break" | "exit" | "halt" | "abort" | "terminate" | "quit" | "kill"
        | "salir" | "arrêter" | "beenden" | "parar"
        | "остановить" | "停止" | "중단" | "إيقاف" | "रोकना"
        => Some("⊣"),

        // continue/next
        "continue" | "next" | "skip" | "pass" | "resume"
        | "continuer" | "continuar" | "weiter" | "следующий"
        | "继续" | "계속" | "استمر" | "जारी रखें"
        => Some("↗"),

        // return
        "return" | "ret" | "retourne" | "devolver" | "zurückgeben" | "retornar"
        | "вернуть" | "返回" | "반환" | "إرجاع" | "वापस करना"
        => Some("→"),

        // ── Geometric ──────────────────────────────────────────────────────────
        "mirror" | "reflect" | "flip" | "reverse" | "inverse" | "invert" | "negate"
        | "transpose" | "dual"
        => Some("⊙"),

        "fold" | "collapse" | "contract" | "compact" | "flatten" | "zip" | "merge"
        | "combine" | "join" | "fuse"
        => Some("⊚"),

        "rotate" | "turn" | "pivot" | "revolve" | "orbit" | "cycle" | "wrap"
        => Some("↷"),

        "translate" | "move" | "shift" | "displace" | "relocate" | "offset"
        | "mover" | "déplacer" | "verschieben" | "переместить" | "移动" | "이동"
        => Some("⇒"),

        "scale" | "resize" | "zoom" | "magnify" | "amplify" | "shrink" | "stretch"
        | "escalar" | "redimensionner" | "skalieren" | "масштаб" | "缩放" | "크기조정"
        => Some("×"),

        // ── Dimensional ────────────────────────────────────────────────────────
        "dim" | "dimension" | "rank" | "depth" | "degree" | "order"
        => Some("Δ"),

        "axis" | "coordinate" | "direction" | "basis" | "vector"
        => Some("Ξ"),

        "plane" | "surface" | "layer" | "sheet" | "face" | "level" | "tier"
        => Some("Π"),

        "space" | "namespace" | "scope" | "realm" | "domain" | "region" | "zone"
        | "espacio" | "espacio_de_nombres" | "espace" | "raum" | "пространство"
        | "空间" | "命名空间" | "공간"
        => Some("□"),

        "hyper" | "infinite" | "unbounded" | "universal" | "all" | "every" | "total"
        | "infinito" | "infini" | "unendlich" | "бесконечный" | "无限" | "무한"
        => Some("∞"),

        // ── Meta / generation ──────────────────────────────────────────────────
        // breed: create/generate/new
        "breed" | "generate" | "create" | "new" | "make" | "build" | "construct"
        | "instantiate" | "allocate" | "init_new" | "factory" | "forge" | "mint"
        | "generar" | "créer" | "erzeugen" | "criar"
        | "создать" | "生成" | "创建" | "생성"
        => Some("⊛"),

        // speak: output/print/display
        "speak" | "print" | "println" | "printf" | "echo" | "say" | "tell"
        | "console" | "display" | "show" | "render" | "paint" | "draw"
        | "format" | "stringify" | "serialize" | "marshal"
        | "imprimir" | "afficher" | "ausgeben" | "imprimir"
        | "вывести" | "输出" | "打印" | "출력"
        => Some("⊘"),

        // morph: transform/convert/adapt
        "morph" | "transform" | "convert" | "coerce" | "adapt" | "map"
        | "transcode" | "reshape" | "rewrite" | "normalize" | "encode" | "decode"
        | "transformar" | "transformer" | "umwandeln"
        | "преобразовать" | "转换" | "변환"
        => Some("∿"),

        // cast: type-cast/coerce
        "cast" | "into" | "typeof" | "instanceof" | "type_of"
        | "upcast" | "downcast" | "reinterpret" | "bitcast"
        => Some("⊗"),

        _ => None,
    }
}

/// Glyph → canonical English keyword (reverse lookup).
pub fn from_glyph(g: &str) -> Option<&'static str> {
    match g {
        "ρ" => Some("pug"),     "β" => Some("bark"),    "ς" => Some("spin"),
        "κ" => Some("cali"),    "ε" => Some("elf"),      "τ" => Some("turk"),
        "φ" => Some("fence"),   "π" => Some("pin"),      "Β" => Some("bone"),
        "Λ" => Some("blob"),    "μ" => Some("biz"),      "∆" => Some("anchor"),
        "∧" => Some("and"),     "∨" => Some("or"),       "¬" => Some("not"),
        "⊕" => Some("xor"),     "⊼" => Some("nand"),     "⊽" => Some("nor"),
        "⊤" => Some("true"),    "⊥" => Some("false"),
        "⁇" => Some("if"),      "∴" => Some("else"),     "⊢" => Some("start"),
        "∀" => Some("for"),     "⋈" => Some("fork"),     "⊃" => Some("branch"),
        "↺" => Some("while"),   "⊣" => Some("break"),    "↗" => Some("continue"),
        "→" => Some("return"),
        "⊙" => Some("mirror"),  "⊚" => Some("fold"),     "↷" => Some("rotate"),
        "⇒" => Some("translate"), "×" => Some("scale"),
        "Δ" => Some("dim"),     "Ξ" => Some("axis"),     "Π" => Some("plane"),
        "□" => Some("space"),   "∞" => Some("hyper"),
        "⊛" => Some("breed"),   "⊘" => Some("speak"),    "∿" => Some("morph"),
        "⊗" => Some("cast"),
        _ => None,
    }
}

/// Returns true if this surface form maps to any glyph.
pub fn is_known(kw: &str) -> bool {
    to_glyph(kw).is_some()
}

/// All canonical keyword → glyph pairs (for help/bench/stats output).
pub const ALL_PAIRS: &[(&str, &str)] = &[
    ("pug","ρ"), ("bark","β"), ("spin","ς"), ("cali","κ"),
    ("elf","ε"), ("turk","τ"), ("fence","φ"), ("pin","π"),
    ("bone","Β"), ("blob","Λ"), ("biz","μ"), ("anchor","∆"),
    ("and","∧"), ("or","∨"), ("not","¬"), ("xor","⊕"), ("nand","⊼"), ("nor","⊽"),
    ("true","⊤"), ("false","⊥"),
    ("if","⁇"), ("else","∴"), ("start","⊢"), ("for","∀"),
    ("fork","⋈"), ("branch","⊃"), ("while","↺"), ("break","⊣"),
    ("continue","↗"), ("return","→"),
    ("mirror","⊙"), ("fold","⊚"), ("rotate","↷"), ("translate","⇒"), ("scale","×"),
    ("dim","Δ"), ("axis","Ξ"), ("plane","Π"), ("space","□"), ("hyper","∞"),
    ("breed","⊛"), ("speak","⊘"), ("morph","∿"), ("cast","⊗"),
];
