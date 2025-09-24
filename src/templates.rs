// Note templates for quick structured note creation

use anyhow::Result;
use chrono::Local;
use std::collections::HashMap;

pub struct NoteTemplate {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub description: String,
    pub content: String,
    #[allow(dead_code)]
    pub tags: Vec<String>,
}

pub struct TemplateManager {
    templates: HashMap<String, NoteTemplate>,
}

impl TemplateManager {
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        // Daily Note
        templates.insert("daily".to_string(), NoteTemplate {
            name: "Daily Note".to_string(),
            description: "Daily journal and task tracking".to_string(),
            content: r#"---
date: {{DATE}}
tags: daily, journal
---

# Daily Note - {{DATE}}

## 🎯 Today's Goals
- [ ]
- [ ]
- [ ]

## 📝 Notes
### Morning Thoughts


### Afternoon Progress


### Evening Reflection


## 🔗 Links to Other Notes
- [[Previous Day]]
- [[Weekly Review]]

## 💡 Ideas & Insights


## 📊 Metrics
- Energy Level: /10
- Productivity: /10
- Mood: /10

## 🙏 Gratitude
1.
2.
3.

---
*Created: {{TIMESTAMP}}*"#.to_string(),
            tags: vec!["daily".to_string(), "journal".to_string()],
        });

        // Meeting Notes
        templates.insert("meeting".to_string(), NoteTemplate {
            name: "Meeting Notes".to_string(),
            description: "Structured meeting notes with action items".to_string(),
            content: r#"---
date: {{DATE}}
time: {{TIME}}
tags: meeting
attendees:
---

# Meeting: {{TITLE}}

## 📍 Details
- **Date:** {{DATE}}
- **Time:** {{TIME}}
- **Location:**
- **Attendees:**

## 📋 Agenda
1.
2.
3.

## 🗣️ Discussion Points


## ✅ Action Items
- [ ] **Person:** Task (Due: )
- [ ] **Person:** Task (Due: )
- [ ] **Person:** Task (Due: )

## 📊 Decisions Made
-

## 🔗 Related Documents
- [[Project Overview]]
- [[Previous Meeting]]

## 🤔 Open Questions


## 📝 Raw Notes


---
*Next Meeting: *
*Created: {{TIMESTAMP}}*"#.to_string(),
            tags: vec!["meeting".to_string()],
        });

        // Project Planning
        templates.insert("project".to_string(), NoteTemplate {
            name: "Project Plan".to_string(),
            description: "Project overview and planning template".to_string(),
            content: r#"---
project: {{PROJECT_NAME}}
status: planning
tags: project, planning
created: {{DATE}}
---

# Project: {{PROJECT_NAME}}

## 🎯 Overview
### Goal


### Success Criteria
-
-
-

## 📅 Timeline
- **Start Date:**
- **Target Completion:**
- **Key Milestones:**
  - [ ] Milestone 1 (Date)
  - [ ] Milestone 2 (Date)
  - [ ] Milestone 3 (Date)

## 👥 Team
- **Project Lead:**
- **Team Members:**
  -
  -

## 🔄 Phases

### Phase 1: Planning
- [ ] Define requirements
- [ ] Create specifications
- [ ] Review and approval

### Phase 2: Development
- [ ]
- [ ]

### Phase 3: Testing
- [ ]
- [ ]

### Phase 4: Deployment
- [ ]
- [ ]

## 📊 Resources
- **Budget:**
- **Tools:**
- **Dependencies:**

## ⚠️ Risks & Mitigation
| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
|      |            |        |            |

## 📝 Notes & Updates


## 🔗 Related Links
- [[Meeting Notes]]
- [[Technical Specs]]
- [[Progress Reports]]

---
*Last Updated: {{TIMESTAMP}}*"#.to_string(),
            tags: vec!["project".to_string(), "planning".to_string()],
        });

        // Research Notes
        templates.insert("research".to_string(), NoteTemplate {
            name: "Research Notes".to_string(),
            description: "Structured research and literature notes".to_string(),
            content: r#"---
topic: {{TOPIC}}
tags: research, literature
date: {{DATE}}
---

# Research: {{TOPIC}}

## 🔍 Research Question


## 📚 Sources
1. **Title:**
   - Author:
   - Year:
   - Link/DOI:
   - Key Points:
     -

2. **Title:**
   - Author:
   - Year:
   - Link/DOI:
   - Key Points:
     -

## 🧠 Key Concepts
### Concept 1
- Definition:
- Importance:
- Related: [[]]

### Concept 2
- Definition:
- Importance:
- Related: [[]]

## 💡 Insights & Analysis


## 🔬 Methodology Notes


## 📊 Data & Evidence


## ❓ Open Questions
-
-

## 🎯 Next Steps
- [ ]
- [ ]

## 🏷️ Tags
#research #{{FIELD}} #literature-review

---
*Research Started: {{DATE}}*
*Last Updated: {{TIMESTAMP}}*"#.to_string(),
            tags: vec!["research".to_string(), "literature".to_string()],
        });

        // Code/Technical Notes
        templates.insert("code".to_string(), NoteTemplate {
            name: "Code Notes".to_string(),
            description: "Technical documentation and code snippets".to_string(),
            content: r#"---
language:
framework:
tags: code, technical
date: {{DATE}}
---

# Code: {{TITLE}}

## 🎯 Purpose


## 💻 Implementation

```{{LANGUAGE}}
// Code here
```

## 🔧 Configuration
```yaml
# Config here
```

## 📝 Usage Example
```{{LANGUAGE}}
// Example usage
```

## 🐛 Known Issues
-

## 🚀 Performance Notes
- Time Complexity: O()
- Space Complexity: O()
- Benchmarks:

## 🔗 Dependencies
-

## 📚 References
- [Documentation]()
- [[Related Note]]

## 🏷️ Tags
#code #{{LANGUAGE}} #{{FRAMEWORK}}

---
*Created: {{TIMESTAMP}}*"#.to_string(),
            tags: vec!["code".to_string(), "technical".to_string()],
        });

        // Book/Article Notes
        templates.insert("reading".to_string(), NoteTemplate {
            name: "Reading Notes".to_string(),
            description: "Notes from books and articles".to_string(),
            content: r#"---
title: {{TITLE}}
author: {{AUTHOR}}
type: book/article
tags: reading, literature
date_read: {{DATE}}
rating: /5
---

# 📚 {{TITLE}}
*by {{AUTHOR}}*

## 📊 Metadata
- **Type:** Book/Article
- **Published:**
- **Pages:**
- **ISBN/DOI:**
- **Rating:** ⭐/5

## 🎯 Key Takeaways
1.
2.
3.

## 📝 Summary


## 💭 Favorite Quotes
> "Quote here" (p. )

> "Another quote" (p. )

## 💡 Personal Reflections


## 🔗 Connections
- Related to [[Other Book]]
- Contradicts [[Some Theory]]
- Supports [[My Project]]

## 🎬 Action Items
- [ ]
- [ ]

## 📖 Chapter Notes
### Chapter 1:
-

### Chapter 2:
-

## 🏷️ Tags
#reading #{{GENRE}} #{{TOPIC}}

---
*Started: {{DATE}}*
*Finished: *
*Notes Created: {{TIMESTAMP}}*"#.to_string(),
            tags: vec!["reading".to_string(), "literature".to_string()],
        });

        Self { templates }
    }

    #[allow(dead_code)]
    pub fn get_template(&self, name: &str) -> Option<&NoteTemplate> {
        self.templates.get(name)
    }

    #[allow(dead_code)]
    pub fn list_templates(&self) -> Vec<(&String, &NoteTemplate)> {
        self.templates.iter().collect()
    }

    pub fn apply_template(&self, template_name: &str, vars: HashMap<String, String>) -> Result<String> {
        let template = self.templates.get(template_name)
            .ok_or_else(|| anyhow::anyhow!("Template not found: {}", template_name))?;

        let mut content = template.content.clone();

        // Add default variables
        let mut all_vars = vars;
        all_vars.insert("DATE".to_string(), Local::now().format("%Y-%m-%d").to_string());
        all_vars.insert("TIME".to_string(), Local::now().format("%H:%M").to_string());
        all_vars.insert("TIMESTAMP".to_string(), Local::now().format("%Y-%m-%d %H:%M:%S").to_string());

        // Replace all variables
        for (key, value) in all_vars {
            let placeholder = format!("{{{{{}}}}}", key);
            content = content.replace(&placeholder, &value);
        }

        Ok(content)
    }

    #[allow(dead_code)]
    pub fn create_custom_template(&mut self, name: String, description: String, content: String, tags: Vec<String>) {
        self.templates.insert(name.clone(), NoteTemplate {
            name: name.clone(),
            description,
            content,
            tags,
        });
    }
}