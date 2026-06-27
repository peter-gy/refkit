from os import PathLike

from refkit_core import (
    BibDocument as BibDocument,
)
from refkit_core import (
    BibEntry as BibEntry,
)
from refkit_core import (
    BibEntryMap as BibEntryMap,
)
from refkit_core import (
    BibField as BibField,
)
from refkit_core import (
    BibFieldMap as BibFieldMap,
)
from refkit_core import (
    Citation as Citation,
)
from refkit_core import (
    CitationGroup as CitationGroup,
)
from refkit_core import (
    Cite as Cite,
)
from refkit_core import (
    Document as Document,
)
from refkit_core import (
    Entry as Entry,
)
from refkit_core import (
    Library as Library,
)
from refkit_core import (
    Locale as Locale,
)
from refkit_core import (
    MissingReferenceError as MissingReferenceError,
)
from refkit_core import (
    RefkitError as RefkitError,
)
from refkit_core import (
    Rendered as Rendered,
)
from refkit_core import (
    RenderedDocument as RenderedDocument,
)
from refkit_core import (
    Style as Style,
)
from refkit_core import (
    build_info as build_info,
)
from refkit_core import (
    build_mode as build_mode,
)

__version__: str

def check_refkit_core_version() -> bool: ...
def cite(
    source: str | PathLike[str],
    citation: str | Cite | CitationGroup,
    *,
    style: str | Style = "apa",
    locale: str | Locale | None = "en-US",
) -> Rendered: ...
def full_bibliography(
    source: str | PathLike[str],
    *,
    style: str | Style = "apa",
    locale: str | Locale | None = "en-US",
) -> Rendered: ...
