# 🧨 [API Reference] 레퍼런스 문서 리팩토링 및 작성 규율 (초강도/상세본)

**"모호한 타입 단어나 파라미터 누락은 유저의 코드를 망치는 죄악이다. 소스 코드를 뜯어보게 만들지 마라."**

현재 Python Binding 모듈 측 API 레퍼런스(`reference/python/*.md` 계열 위주)는 기계적이고 엄격한 명세표(Table) 기준 없이, 그저 "이 파라미터는 이런 동작을 수행합니다" 수준의 문장형 나열로 서술되는 나쁜 경향이 있습니다. API 레퍼런스는 서술형 산문 에세이가 아닙니다. 100% 무결성을 탑재한 결벽증적인 '사전(Dictionary)'이어야만 합니다.

## 🚨 1. 현재 문서 생태계의 치명적 결함 및 해부

Python 생태계의 경우 C++이나 Rust와 다르게 컴파일 타임에 철저한 타입 추론 체크가 가이드되지 않습니다. 그 말인즉슨, Core 엔진에서 PyO3 바이너리를 거쳐 바인딩된 Lynxes 객체나 데이터들이 파이썬으로 내던져질 때 그 반환 타입이 '진박하게 무엇인지'는 온전히 문서 환경이 독박으로 책임지고 안내해야 합니다.

**결함 1: 매우 불쾌한 암묵적 타입 (Implicit Types) 두루뭉술 처리법**
*   가령 반환값에 대해 "이 함수는 노드 리스트를 반환합니다." -> 이게 순수 파이썬의 `list` 객체입니까 아니면 Numpy 기반 `ndarray` 입니까, 아니면 메모리가 제로 보존된 Apache Arrow `Array` 객체입니까? 현재 레퍼런스 문서는 이 선명한 경계선을 매우 흐릿하게 포장하고 있습니다. 데이터 분석가에게 메모리 타입은 생명과 직결됩니다.

**결함 2: Exception/Error 명세의 무책임한 완전 백지화**
*   예를 들어, Python 단에서 `GraphFrame.shortest_path("A", "B")` 를 야심차게 호출할 때 만일 DB에 "A"라는 노드가 없다면 엔진은 무슨 에러로 응답합니까? 파이썬 빌트인 `KeyError`가 뜹니까, 아니면 내부 Rust 래퍼로 포장된 `GFError::NodeNotFound`가 발생합니까? 현재 참고되고 있는 문서들에는 "어떨 때 앱이 터진다"를 예측할 수 있는 `Raises (예외 발생)` 블록이 단 한 곳도 존재하지 않는 위험 기류가 있습니다.

## 🛠 2. 즉각적인 리팩토링 지시 및 문서 표준 규격

### Rule 1: 100% Parameter 컴포넌트 테이블 작성 의무화 (Table Format)
모든 노출된 `pub` 단위 클래스나 함수 API, 메서드 설명의 몸통 밑에는 예외를 불허하고 아래의 5열 체계 테이블 렌더 마크다운이 존재해야 합니다. 단 하나의 필드 누락도 용납하지 않습니다.

| Parameter Name | Data Type | Required / Optional | Default Value | Description |
| :--- | :--- | :--- | :--- | :--- |
| `name` | `str` | Required | - | 검색/확장 대상 노드를 식별하는 유일키(Primary ID) 문자열 |
| `hops` | `int` | Optional | `1` | 그래프 바깥쪽으로 확장 전개할 이웃 반경 최대 수치 범위 (Limit bounds: 1~1000) |

### Rule 2: 반환(Return) 타입 구조에서 '제로카피(Zero-copy)' 등 물리적 본질 명시
*   파이썬 측 반환값이 백엔드의 Arrow Columnar 메모리를 물리적으로 직접 참조 중인 얇은 뷰 슬라이스(View Layer) 타입인지, 아니면 파이썬 Native 메모리로 오버헤드를 일으켜 복사된(Copied/Cloned) 데이터 덩어리 무더기인지를 `Returns:` 섹션 항목에 강제성 있게 적시하십시오. 퍼포먼스와 메모리를 쥐어짜는 핵심 사용자에게 이 부분은 최우선 판단 정보 기준이 됩니다.

### Rule 3: Error & Exception 구조와 GFError 객체의 명확한 1:1 매핑 선언
*   Rust Core에 하드코딩 되어 존재하는 `GFError` enum의 에러 컨디션들(`NodeNotFound`, `ParseError`, `BoundsExceeded` 등)이 각각 어느 파이썬 메서드 함수를 두드릴 때 시스템 예외(Exception Error)로 던져져서 프로그램을 멈추게 하는지, 함수 끝 바닥에 `Raises:` 조항으로 필수로 서술 등록하세요.

## 📝 3. (예시) 기존 불량 문서들을 이렇게 모조리 리팩토링하십시오.

**[AS-IS] (과거의 무능하고 불성실한 명세 방식)**
> `def expand(edge_type: str, hops: int)`
> 주어진 엣지 파라미터 타입에 따라 연결된 노드들을 쭉 탐색해 가져옵니다.

**[TO-BE] (사전처럼 냉정하고 완벽하게 파해쳐진 명세 방식)**
> ### `expand(edge_type: str, hops: int = 1) -> LazyGraphFrame`
> 
> 즉시 그래프 메모리를 스캔하지 않고, 주어진 엣지 타입에 기반하여 백엔드 상으로 `LogicalPlan` 확장을 대기 스케줄링하는 핵심 지연 연산(Lazy 연산) 메서드입니다.
> 
> **Parameters 상세:**
> | Name | Type | Required | Default | Description |
> | :--- | :--- | :--- | :--- | :--- |
> | `edge_type` | `str` | Required | - | 쿼리를 타고 이동할 타겟 엣지의 유일한 `_type` 식별자. ※ 주의: 인자값 대소문자를 엄격히 구분합니다. |
> | `hops` | `int` | Optional | `1` | 쿼리할 총 반경 홉 확장 수치. (입력 허용 최소치: 1, 퍼포먼스 보호를 위한 권장 한계치: 5 이하) |
> 
> **Returns (반환값의 물리적 본질):** 
> *   `LazyGraphFrame`: 호출 시 CPU I/O 연산이 바로 발동되지 않으며 후행 필터 체이닝이 가능하도록 설계된 지연(Lazy) 컨텍스트 래퍼 객체를 반환합니다. 메모리 복사 발생량: Zero(0)
> 
> **Raises (치명 예외 유발 사항):**
> *   `InvalidHopRangeError`: 만약 `hops` 파라미터 값이 1 미만의 음수이거나, 내부 스택 정수 범위를 초과하는 Integer 값을 투입했을 경우 호출 즉각 발생합니다.
> *   `GFError::SchemaMismatch`: 데이터에 선언되지 않은 허위 `edge_type`을 기재 시 발생합니다.
