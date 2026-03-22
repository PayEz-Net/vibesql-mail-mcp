# ACP Workflow Templates — New Project Flow Spec

Source: BAPert mail #889, 2/23/2026

## What It Is

When you start a new task, you pick a workflow template that defines:
- Which agents are involved
- What steps they follow
- How work flows between them (who hands off to who)
- What gets created (branches, files, mail threads)

## Where It Lives

In the TUI: new screen accessed via [W]orkflow from the nav bar.
In the visual office: button in the bottom toolbar next to "+ Agent".
Both create the same workflow — TUI creates mail threads, office shows agents animating through steps.

## Built-In Workflow Templates

### 1. Quick Change
- Agents: Engineer only
- Steps:
  1. Engineer receives task description
  2. Engineer implements
  3. Done
- Use when: small fix, one person, no review needed

### 2. Fix Bug
- Agents: Engineer, Designer (optional)
- Steps:
  1. Engineer receives bug report
  2. Engineer investigates and fixes
  3. Engineer self-reviews
  4. Done
- Use when: bug report, needs investigation

### 3. Spec and Build
- Agents: Strategist, Engineer
- Steps:
  1. Strategist receives feature request, writes spec
  2. Strategist mails spec to Engineer
  3. Engineer reviews spec, asks clarifying questions (mail thread)
  4. Engineer implements
  5. Engineer mails Strategist: "done, here's what I built"
  6. Strategist reviews against spec
  7. Done
- Use when: new feature that needs a spec first

### 4. Full Team Workflow
- Agents: Strategist, Engineer, Designer
- Steps:
  1. Strategist receives request, writes product spec
  2. Strategist mails spec to Designer and Engineer
  3. Designer reviews UX implications, mails feedback
  4. Engineer reviews technical feasibility, mails feedback
  5. Strategist incorporates feedback, mails final spec
  6. Engineer implements
  7. Designer reviews UX of implementation
  8. Engineer addresses Designer feedback
  9. Strategist does final acceptance review
  10. Done
- Use when: significant feature, needs full team input

## New Project Screen (TUI)

Template picker with j/k navigation, Enter to select. After selecting:
- Project name (text input)
- Description (text area)
- Shows agents and steps for selected workflow
- Ctrl+S to Create & Start

## What "Create & Start" Does

1. Creates a mail thread with the project name as subject
2. Sends the first mail in the workflow:
   - Quick Change: mails Engineer directly
   - Spec and Build: mails Strategist with the task description
   - Full Team: mails Strategist with CC to Engineer and Designer
3. Stores the workflow state in a project record
4. Each step completion triggers the next mail automatically (or manually — user advances steps)

## Server Side

### New Tables

```sql
CREATE TABLE projects (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  workflow TEXT NOT NULL,        -- 'quick_change', 'fix_bug', 'spec_build', 'full_team', or custom
  status TEXT DEFAULT 'active',  -- 'active', 'completed', 'archived'
  current_step INTEGER DEFAULT 1,
  total_steps INTEGER NOT NULL,
  thread_id TEXT NOT NULL,       -- links to mail thread
  created_by TEXT NOT NULL,      -- agent who created it
  created_at TIMESTAMPTZ DEFAULT NOW(),
  completed_at TIMESTAMPTZ
);

CREATE TABLE project_steps (
  id SERIAL PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  step_number INTEGER NOT NULL,
  agent_name TEXT NOT NULL,      -- who does this step
  action TEXT NOT NULL,          -- 'write_spec', 'review', 'implement', 'feedback', etc.
  description TEXT,
  status TEXT DEFAULT 'pending', -- 'pending', 'active', 'completed', 'skipped'
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ
);

CREATE INDEX idx_projects_status ON projects(status);
CREATE INDEX idx_project_steps_project ON project_steps(project_id);
```

### New Endpoints

```
POST /v1/mail/projects              -- create project + steps
GET  /v1/mail/projects              -- list active projects
GET  /v1/mail/projects/{id}         -- project detail with steps
POST /v1/mail/projects/{id}/advance -- advance to next step
POST /v1/mail/projects/{id}/complete -- mark project done
```

## Custom Workflows

Users can define custom workflow templates as JSON files:

`~/.vibesql-mail/workflows/code-review.json`:
```json
{
  "name": "Code Review",
  "description": "Submit code for peer review",
  "agents": ["Engineer", "Designer"],
  "steps": [
    {"agent": "Engineer", "action": "submit", "description": "Submit code for review"},
    {"agent": "Designer", "action": "review", "description": "Review code, mail feedback"},
    {"agent": "Engineer", "action": "address", "description": "Address feedback"},
    {"agent": "Designer", "action": "approve", "description": "Final approval"}
  ]
}
```

## Visual Office Integration

When a project is active:
- Current step's agent has a colored highlight at their desk
- Speech bubble shows current step description
- Completed steps show checkmark above agent
- Activity feed shows step transitions

## Implementation Order

1. Projects table + endpoints (server side)
2. Workflow screen in TUI (client side)
3. Built-in 4 templates
4. Project detail view (show steps, current progress)
5. Custom workflow JSON loading
6. Visual office integration (later, with pixel agents work)
