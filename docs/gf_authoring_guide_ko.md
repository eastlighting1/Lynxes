# Lynxes `.gf` 파일 작성 명세 및 가이드

`.gf` 파일은 Lynxes에서 사용하는 그래프 데이터를 사람이 읽고 쓰기 쉽게 만든 텍스트 포맷입니다. 
이 문서는 `.gf` 파일을 작성하는 방법과 문법(Syntax) 규칙을 요약한 명세/작성 가이드입니다. 

---

## 1. 기본 구조

`.gf` 파일은 일반적으로 다음과 같은 순서로 작성됩니다:
1. `@meta` 등 파일 제어 구문 (선택 사항)
2. `node` 및 `edge` 스키마 정의 (선택 사항)
3. 노드(Node) 선언 (필수)
4. 엣지(Edge) 선언 (필수)

> 주의: 빈 파일이나 메타데이터/스키마만 있는 파일은 유효하지 않으며, 최소 1개 이상의 노드나 엣지 데이터 선언이 있어야 합니다.

---

## 2. 노드(Node) 작성법

노드는 소괄호 `()`로 감싸서 표현합니다. 소괄호 안의 첫 번째 이름(식별자)은 노드의 로컬 고유 ID가 됩니다.

### 기본 노드 선언
```gf
(alice)
```

### 라벨(Label) 추가
콜론 `:` 기호 뒤에 라벨을 지정합니다. `|`를 사용해 여려 개의 라벨을 지정할 수 있습니다.
```gf
(alice: Person)
(bob: Person|Employee)
```

### 속성(Property) 추가
중괄호 `{}` 안에 `키: 값` 형태로 속성을 지정합니다. 여러 속성은 쉼표(`,`)로 구분합니다.
```gf
(alice: Person { name: "Alice", age: 30, city: "Seoul" })
```

---

## 3. 엣지(Edge) 작성법

엣지는 화살표 형태로 연결의 방향성을 시각적으로 표현합니다. 대괄호 `[]` 안에 엣지 타입을 지정하며, 엣지 타입은 필수입니다. 엣지의 양 끝은 소괄호에서 선언한 노드의 고유 ID를 참조합니다.

### 엣지 방향
- **정방향 (Outbound):** `-[TYPE]->`
- **역방향 (Inbound):** `<-[TYPE]-`
- **양방향 (Bidirectional):** `<-[TYPE]->`
- **무방향 (Undirected):** `--[TYPE]--`

### 작성 예시
```gf
# 정방향 엣지 (alice 가 bob 을 안다)
alice -[KNOWS]-> bob

# 양방향/무방향 엣지
alice <-[FRIENDS]-> bob
alice --[COWORKER]-- bob
```

### 엣지 속성(Property) 추가
엣지에 대한 속성은 전체 엣지 선언 **뒤에** 중괄호 `{}`를 사용해 추가합니다.
```gf
alice -[KNOWS]-> bob { since: 2020, weight: 0.9 }
```
> 주의: `alice -[KNOWS { since: 2020 }]-> bob` 과 같이 괄호 안에 속성을 삽입하는 문법은 허용되지 않습니다.

---

## 4. 스키마(Schema) 정의

`node`와 `edge` 데이터의 구조를 명시적으로 검증하고 제어하기 위해 스키마를 정의할 수 있습니다.

### 노드 스키마
```gf
node Person {
    name: String @index
    age: Int?
}

node Employee extends Person {
    employee_id: String @unique
}
```
* **상속**: `extends` 키워드로 다른 노드 스키마를 상속받을 수 있습니다.
* **선택적 속성**: 타입 뒤에 `?`를 붙여 속성이 선택(Optional) 값임을 나타냅니다.
* **디렉티브**: `@index`, `@unique`, `@default(값)` 등 제어 구문을 추가할 수 있습니다.

### 엣지 스키마
```gf
edge KNOWS {
    since: Int?
    weight: Float @default(1.0)
}
```
* 엣지 스키마는 상속(`extends`)을 지원하지 않습니다.

---

## 5. 메타데이터(Metadata)와 기타 구문

전체 파일에 대한 메타정보를 포함할 때 사용합니다. 파일 최상단에 추가하는 것을 권장합니다.

```gf
@meta {
    name: "social_graph",
    version: "1.0",
    created: 2026-04-20
}
```

---

## 6. 지원 데이터 타입 및 리터럴 규칙

`.gf` 포맷에서 값을 작성할 때 다음의 포맷 규칙을 준수합니다:

* **문자열 (String):** 큰따옴표 사용. `"Hello", "Alice"`
* **정수 (Int):** 10진수 숫자. `42`, `-7` 
* **실수 (Float):** 소수점 포함 숫자. `3.14`, `-0.5`
* **불리언 (Bool):** `true`, `false` (소문자만 유효)
* **날짜 (Date):** `YYYY-MM-DD` 형식 (예: `2026-04-20`)
* **날짜시간 (DateTime):** `YYYY-MM-DDTHH:MM:SS` 형식
* **리스트 (List):** 대괄호 사용. `[1, 2, 3]`, `["a", "b"]`
* **Null:** `null` (소문자만 유효)

---

## 7. 권장 작성 스타일 (모범 사례)

```gf
# 1. 파일 상단에 메타데이터 정의
@meta {
    name: "employee_graph",
    version: "1.0"
}

# 2. 데이터 형식 규격을 위한 스키마 정의
node Person {
    name: String
    age: Int?
}
node Company {
    name: String
}
edge WORKS_AT {
    role: String
    since: Int
}

# 3. 노드 인스턴스 나열
(alice: Person { name: "Alice", age: 30 })
(bob: Person { name: "Bob", age: 25 })
(acme: Company { name: "Acme Corp" })

# 4. 연관 엣지 나열
alice -[WORKS_AT]-> acme { role: "Engineer", since: 2023 }
bob -[WORKS_AT]-> acme { role: "Designer", since: 2024 }
alice -[KNOWS]-> bob {}
```

---

## 8. 제약 및 주의사항

* **예약어:** 속성명 중 `_` (밑줄)로 시작하는 식별자(`_id`, `_label`, `_src`, `_dst`, `_type`, `_direction`)는 시스템 내부 컬럼으로 예약되어 있어, 직접 작성할 수 없습니다.
* **인코딩:** 파일은 반드시 **UTF-8 인코딩**이어야 합니다.
* **파싱 규칙:** 공백이나 줄바꿈은 파싱에 영향을 미치지 않습니다. 하지만 가독성을 위해 노드와 엣지를 선언할 때 규칙적인 들여쓰기와 줄바꿈을 활용하는 것이 좋습니다.
* **주석:** `#` 기호를 사용해 단일 행 주석을 작성할 수 있습니다.
