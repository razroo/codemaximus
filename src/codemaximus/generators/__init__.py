from codemaximus.generators.enterprise_java import EnterpriseJavaGenerator
from codemaximus.generators.enterprise_python import EnterprisePythonGenerator
from codemaximus.generators.javascript import JavaScriptGenerator
from codemaximus.generators.go_slop import GoSlopGenerator
from codemaximus.generators.generic import GenericGenerator

ALL_GENERATORS = [
    EnterpriseJavaGenerator(),
    EnterprisePythonGenerator(),
    JavaScriptGenerator(),
    GoSlopGenerator(),
    GenericGenerator(),
]

JAVA_GEN, PY_GEN, JS_GEN, GO_GEN, GENERIC_GEN = ALL_GENERATORS

LANG_MAP = {
    "java": [JAVA_GEN],
    "python": [PY_GEN],
    "js": [JS_GEN],
    "javascript": [JS_GEN],
    "go": [GO_GEN],
    "generic": [GENERIC_GEN],
    "all": ALL_GENERATORS,
}


def get_generators(lang: str) -> list:
    return LANG_MAP.get(lang, ALL_GENERATORS)
