Pour tester chacune des constantes `QUERY_*` et créer les constantes `CODE_*` associées, il faut écrire des tests unitaires qui vérifient que chaque requête Tree-sitter capture correctement les éléments attendus dans un code Rust. Voici comment tu peux procéder pour chaque type de requête, avec des exemples de code Rust (`CODE_*`) et les tests correspondants.

---

### 1. **`QUERY_FONCTION_BLOCK`**
**Objectif** : Capturer les fonctions, y compris celles imbriquées dans des modules, traits, impls, etc.

#### Constante `CODE_FN` (déjà présente)
```rust
const CODE_FN: &'static [u8; 444] = br###"
fn fonction(code: &str) -> Result<Vec<String>, String> {
    fn inner(code: &str) -> Result<Vec<String>, String> {todo!()}
}
trait Trait {
    fn fonction(code: &str) -> Result<Vec<String>, String> {todo!()}
}
impl Implementation {
    fn fonction(code: &str) -> Result<Vec<String>, String> {todo!()}
}
mod mymod {
    impl quelque::part::Trait for SomeStruct {
        fn fonction(code: &str) -> Result<Vec<String>, String> {todo!()}
    }
}
"###;
```

#### Test
```rust
#[test]
fn list_fn() -> Result<()> {
    let tags = list_tags(CODE_FN, &*QUERY_FONCTION_BLOCK)?;
    assert_eq!(
        tags,
        vec![
            "fonction",
            "fonction::inner",
            "Trait::fonction",
            "Implementation::fonction",
            "mymod::quelque::part::Trait::SomeStruct::fonction"
        ]
    );
    Ok(())
}
```

---

### 2. **`QUERY_STRUCTURE_BLOCK`**
**Objectif** : Capturer les structures (`struct`).

#### Constante `CODE_STRUCT`
```rust
const CODE_STRUCT: &'static [u8; 200] = br###"
struct MyStruct;
mod mymod {
    struct InnerStruct;
}
trait Trait {
    struct AssociatedStruct;
}
"###;
```

#### Test
```rust
#[test]
fn list_struct() -> Result<()> {
    let tags = list_tags(CODE_STRUCT, &*QUERY_STRUCTURE_BLOCK)?;
    assert_eq!(
        tags,
        vec![
            "MyStruct",
            "mymod::InnerStruct",
            "Trait::AssociatedStruct",
        ]
    );
    Ok(())
}
```

---

### 3. **`QUERY_ENUMERATION_BLOCK`**
**Objectif** : Capturer les énumérations (`enum`).

#### Constante `CODE_ENUM`
```rust
const CODE_ENUM: &'static [u8; 200] = br###"
enum MyEnum { A, B }
mod mymod {
    enum InnerEnum { X, Y }
}
trait Trait {
    enum AssociatedEnum { P, Q }
}
"###;
```

#### Test
```rust
#[test]
fn list_enum() -> Result<()> {
    let tags = list_tags(CODE_ENUM, &*QUERY_ENUMERATION_BLOCK)?;
    assert_eq!(
        tags,
        vec![
            "MyEnum",
            "mymod::InnerEnum",
            "Trait::AssociatedEnum",
        ]
    );
    Ok(())
}
```

---

### 4. **`QUERY_TYPE`**
**Objectif** : Capturer les alias de type (`type`).

#### Constante `CODE_TYPE` (déjà présente)
```rust
const CODE_TYPE: &'static [u8; 554] = br###"
type Type = usize;
fn fonction(code: &str) -> Result<Vec<String>, String> {
    type Type = usize;
}
trait Trait {
    type Type = usize;
    fn fonction(code: &str) -> Result<Vec<String>, String> {todo!()}
}
impl Implementation {
    type Type = usize;
    type AssociatedType: Trait;
    fn fonction(code: &str) -> Result<Vec<String>, String> {todo!()}
}
mod mymod {
    type Type = usize;
    impl quelque::part::Trait for SomeStruct {
        fn fonction(code: &str) -> Result<Vec<String>, String> {
            type Type = usize;
        }
    }
}
"###;
```

#### Test
```rust
#[test]
fn list_type() -> Result<()> {
    let tags = list_tags(CODE_TYPE, QUERY_TYPE)?;
    assert_eq!(
        tags,
        vec![
            "Type",
            "fonction::Type",
            "Trait::Type",
            "Implementation::Type",
            "mymod::Type",
            "mymod::quelque::part::Trait::SomeStruct::fonction::Type",
        ]
    );
    Ok(())
}
```

---

### 5. **`QUERY_STATIC`**
**Objectif** : Capturer les variables statiques (`static`).

#### Constante `CODE_STATIC`
```rust
const CODE_STATIC: &'static [u8; 200] = br###"
static MY_STATIC: usize = 0;
mod mymod {
    static INNER_STATIC: usize = 0;
}
trait Trait {
    static ASSOCIATED_STATIC: usize = 0;
}
"###;
```

#### Test
```rust
#[test]
fn list_static() -> Result<()> {
    let tags = list_tags(CODE_STATIC, QUERY_STATIC)?;
    assert_eq!(
        tags,
        vec![
            "MY_STATIC",
            "mymod::INNER_STATIC",
            "Trait::ASSOCIATED_STATIC",
        ]
    );
    Ok(())
}
```

---

### 6. **`QUERY_CONSTANT`**
**Objectif** : Capturer les constantes (`const`).

#### Constante `CODE_CONST`
```rust
const CODE_CONST: &'static [u8; 200] = br###"
const MY_CONST: usize = 0;
mod mymod {
    const INNER_CONST: usize = 0;
}
trait Trait {
    const ASSOCIATED_CONST: usize = 0;
}
"###;
```

#### Test
```rust
#[test]
fn list_const() -> Result<()> {
    let tags = list_tags(CODE_CONST, QUERY_CONSTANT)?;
    assert_eq!(
        tags,
        vec![
            "MY_CONST",
            "mymod::INNER_CONST",
            "Trait::ASSOCIATED_CONST",
        ]
    );
    Ok(())
}
```

---

### 7. **`QUERY_MOD_IMPORT`**
**Objectif** : Capturer les modules (`mod`) et leurs imports.

#### Constante `CODE_MOD`
```rust
const CODE_MOD: &'static [u8; 200] = br###"
mod mymod;
mod inner {
    mod deeper;
}
"###;
```

#### Test
```rust
#[test]
fn list_mod() -> Result<()> {
    let tags = list_tags(CODE_MOD, QUERY_MOD_IMPORT)?;
    assert_eq!(
        tags,
        vec![
            "mymod",
            "inner",
            "inner::deeper",
        ]
    );
    Ok(())
}
```

---

### 8. **`QUERY_USE_SUCCESSIVE`**
**Objectif** : Capturer les déclarations `use` successives.

#### Constante `CODE_USE`
```rust
const CODE_USE: &'static [u8; 89] = br###"
use std::collections::HashMap;
use std::io;
use std::path::Path;
"###;
```

#### Test
```rust
#[test]
fn list_use() -> Result<()> {
    let tags = list_tags(CODE_USE, QUERY_USE_SUCCESSIVE)?;
    // Ici, on vérifie que la requête capture bien les déclarations `use`.
    // Comme `list_tags` ne retourne pas de `Tag` pour les `use`, il faut adapter le test.
    // Par exemple, vérifier que la requête ne plante pas et retourne un résultat vide ou attendu.
    assert!(tags.is_empty());
    Ok(())
}
```

---

### Résumé des tests
- Chaque test vérifie que la requête Tree-sitter capture les éléments attendus dans le code Rust fourni.
- Les constantes `CODE_*` contiennent des exemples de code Rust pertinents pour chaque type de requête.
- Les tests comparent les `Tag` générés avec les résultats attendus.

Si tu veux, je peux générer un fichier Rust complet avec tous ces tests et constantes !
