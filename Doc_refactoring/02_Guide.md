# 🧨 [Guide/Tutorial] 가이드 문서 리팩토링 지시서 (초강도/상세본)

**"뉴비에게 컴파일러 에러를 보게 하는 것은 범죄입니다. 100% 한 번에 성공하는 외길(Happy Path)을 뚫어놓으십시오."**

초보자나 입문자를 위한 가이드(`install.md`, `quickstart/python.md` 등)는 이 제품을 대하는 유저의 생애 '첫인상'입니다. 현재 문서들은 대상 독자층 분리에 철저히 실패하고 있으며, 기능에 대한 가벼운 체험만 주고 감동적인 경험(Aha-moment)을 주지 못합니다.

## 🚨 1. 현재 문서의 치명적 결함 및 철저한 해부

**결함 1: `docs/install.md` 단계 설계의 치명적 붕괴 (난이도 급발진)**
현재 `install.md` 파일의 41번 라인 부근을 보면, 돌연 `maturin develop --release` 같은 Rust/C++ 네이티브 빌드 환경 구축 요구사항이 아무렇지도 않게 튀어나옵니다. 일반적인 Python 유저가 `uv load`, `cargo 빌드 툴체인 구축` 같은 문구를 마주하는 순간 탭을 닫아버리고 제품 도입을 백지화합니다.

**결함 2: `docs/quickstart/python.md`의 무책임하게 생략된 출력값 명시**
퀵스타트 문서의 47번 라인 이하에서 명시된 쿼리 체이닝(`.expand(edge_type="KNOWS", hops=2).collect()`)을 설명하는 코드 블록을 보십시오. "이 코드는 3가지 작업을 합니다"라고 설명하지만, 정작 코드 실행 후 **터미널이나 주피터 쉘에 정확히 무엇이 찍혀야 정상인지**가 단 한 줄도 없습니다. 콘솔 출력 검증 결과가 문서 상에 없으므로, 유저는 본인이 제품을 제대로 작동시켰는지 확신할 수 없게 됩니다.

## 🛠 2. 즉각적인 리팩토링 지시 및 작성 규칙

### Rule 1: `install.md`의 철저한 계급(독자 타겟) 분리
*   해당 문서를 두 그룹으로 철저히 격리시키거나 아예 다른 파일로 분할하십시오.
*   **일반 Python 유저용:** 오직 `pip install lynxes` 및 `uv add lynxes` 명령어만을 첫 페이지 전면에 탭 UI로 배치하세요.
*   **엔진 기여자 및 해커용 (Source Build):** "이 하부 섹션부터는 프로젝트 코어 엔진을 수정하거나 로컬 컴파일을 할 분들을 위한 Rust 툴체인 타겟 문서입니다"라는 `> [!WARNING]` 경고문을 거대하게 부착하여, 일반 유저가 읽다가 지레 겁먹고 도망가게 하는 현상을 막으십시오.

### Rule 2: 모든 퀵스타트 코드에 '출력 기대값(Expected Output)' 하드코딩
예외 없이 모든 실습용 Python/CLI 코드 블록 바로 아래에는, 실행 직후 터미널에 노출되는 예상 아웃풋(텍스트 구조물)을 그대로 하드코딩하십시오.
*   **수정 지시:**
    ```python
    print("expanded nodes:", result.node_count())
    ```
    이 구문 아래에 주석이나 텍스트 아웃풋 형식으로 실제 반환값(`#> expanded nodes: 42`)을 박아두어, 유저가 자신의 모니터와 문서를 비교하며 "성공했다"는 검증(Verify) 도파민을 얻게 하세요.

### Rule 3: 환경 불치증 유발 요소 원천 제거 (100% 작동 보장 설계)
*   현재 `quickstart/python.md` 19라인에는 "If you are working from a GitHub checkout, reuse the shared example file..." 이라며 유저가 직접 테스트 파일을 뒤져서 알아서 경로를 찾도록 방치합니다. 이것이 실패 확률을 50% 이상 치솟게 만듭니다.
*   **결정적 지시:** 첫 번째 튜토리얼에서는 파일을 찾게 하지 마십시오. 문서 내부에서 파이썬 코드로 `lx.from_records(...)` 와 같은 인-메모리 딕셔너리를 사용하여 단 5줄 만에 데이터를 만들고 100% 에러 없이 구동되게 설계하거나, 코드를 복붙하는 즉시 `curl github.com/.../social.gf`로 샘플 파일을 강제 다운로드 시키는 방어 기제를 탑재하십시오.

## 📝 3. (예시) 이렇게 전면 교체하십시오.

**[AS-IS] (기존 Python Quickstart 의 추상적이고 무책임한 끝맺음)**
> At this point you have an eager `GraphFrame`. You can inspect counts immediately and call algorithms directly on it.

**[TO-BE] (출력값을 직접 비교/확인시켜주는 철저한 친절함)**
> 코드를 복사하여 실행했다면 콘솔 창에서 아래와 같은 정확한 결과를 육안으로 확인하셔야 합니다!
> ```bash
> nodes: 3
> edges: 2
> density: 0.666
> node columns: ['_id', '_label', 'name', 'age']
> ```
> 만약 모니터에 위 결과 대신 `FileNotFoundError`나 `GFError::Parse` 에러가 발생한다면, 당황하지 말고 1단계로 돌아가 명령어를 쳤던 디렉토리에 `example_simple.gf` 화일이 정상적으로 위치해 있는지 경로를 다시 점검하세요.
