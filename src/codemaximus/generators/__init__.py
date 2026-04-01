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

LANG_MAP = {
    "java": [EnterpriseJavaGenerator()],
    "python": [EnterprisePythonGenerator()],
    "js": [JavaScriptGenerator()],
    "javascript": [JavaScriptGenerator()],
    "go": [GoSlopGenerator()],
    "generic": [GenericGenerator()],
    "all": ALL_GENERATORS,
}


def get_generators(lang: str) -> list:
    return LANG_MAP.get(lang, ALL_GENERATORS)
