use dioxus::prelude::*;
use crate::backend::{AppCmd, dag::{DagPayload, CourseCategory}};
use crate::components::AppState;

#[component]
pub fn EducationComponent() -> Element {
    let mut app_state = use_context::<AppState>();
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();
    
    let mut active_tab = use_signal(|| "courses".to_string());
    let mut show_create_course = use_signal(|| false);
    let mut viewing_course = use_signal(|| None::<crate::backend::dag::DagNode>);
    
    // Course form state
    let mut course_title = use_signal(|| String::new());
    let mut course_description = use_signal(|| String::new());
    let mut course_content = use_signal(|| String::new());
    let mut course_category = use_signal(|| "CivicLiteracy".to_string());
    
    // Exam taking state
    let mut current_question_idx = use_signal(|| 0usize);
    
    // Fetch data on mount
    let cmd_tx_effect = cmd_tx.clone();
    use_effect(move || {
        let _ = cmd_tx_effect.send(AppCmd::FetchCourses);
        let _ = cmd_tx_effect.send(AppCmd::FetchExams);
        let _ = cmd_tx_effect.send(AppCmd::FetchMyCertifications);
    });
    
    let cmd_tx_create = cmd_tx.clone();
    let on_create_course = move |_| {
        if !course_title().is_empty() {
            let _ = cmd_tx_create.send(AppCmd::CreateCourse {
                title: course_title(),
                description: course_description(),
                content: course_content(),
                category: course_category(),
                prerequisites: vec![],
            });
            course_title.set(String::new());
            course_description.set(String::new());
            course_content.set(String::new());
            show_create_course.set(false);
            let _ = cmd_tx_create.send(AppCmd::FetchCourses);
        }
    };
    
    // Start exam handler
    let start_exam = move |exam_node: crate::backend::dag::DagNode| {
        if let DagPayload::Exam(exam) = &exam_node.payload {
            let question_count = exam.questions.len();
            app_state.active_exam.set(Some(exam_node));
            app_state.exam_answers.set(vec![None; question_count]);
            current_question_idx.set(0);
            app_state.exam_result.set(None);
        }
    };
    
    // Close exam modal
    let close_exam = move |_| {
        app_state.active_exam.set(None);
        app_state.exam_answers.set(vec![]);
        app_state.exam_result.set(None);
        current_question_idx.set(0);
    };
    
    // Submit exam
    let cmd_tx_submit = cmd_tx.clone();
    let submit_exam = move |_| {
        let active = app_state.active_exam.read();
        if let Some(exam_node) = active.as_ref() {
            let answers = app_state.exam_answers.read();
            let answer_vec: Vec<usize> = answers.iter().map(|a| a.unwrap_or(0)).collect();
            let _ = cmd_tx_submit.send(AppCmd::SubmitExam {
                exam_id: exam_node.id.clone(),
                answers: answer_vec,
            });
        }
    };

    // If viewing a course, show detail view
    if let Some(course_node) = viewing_course() {
        return rsx! {
            CourseDetailComponent {
                course_node: course_node,
                on_back: move |_| viewing_course.set(None),
                on_take_exam: start_exam
            }
        };
    }

    rsx! {
        div { class: "page-container py-8 animate-fade-in",
            
            // Header
            div { class: "page-header",
                div { class: "flex justify-between items-center",
                    div {
                        h1 { class: "page-title", "📚 Education Portal" }
                        p { class: "text-[var(--text-secondary)] mt-1", "Learn, earn certifications, and build your skills" }
                    }
                    div { class: "flex gap-2",
                        if active_tab() == "courses" {
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| show_create_course.set(!show_create_course()),
                                if show_create_course() { "Cancel" } else { "+ Create Course" }
                            }
                        }
                    }
                }
            }

            // Tabs
            div { class: "flex gap-2 mb-6",
                button {
                    class: if active_tab() == "courses" { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| active_tab.set("courses".to_string()),
                    "📖 Courses"
                }
                button {
                    class: if active_tab() == "exams" { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| active_tab.set("exams".to_string()),
                    "📝 Exams"
                }
                button {
                    class: if active_tab() == "certifications" { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| active_tab.set("certifications".to_string()),
                    "🏆 My Certifications"
                }
            }

            // Create Course Form
            if show_create_course() && active_tab() == "courses" {
                div { class: "panel mb-6",
                    div { class: "panel-header",
                        h2 { class: "panel-title", "Create New Course" }
                    }
                    div { class: "grid gap-4",
                        div { class: "form-group",
                            label { class: "form-label", "Title" }
                            input {
                                class: "input",
                                placeholder: "Course title...",
                                value: "{course_title}",
                                oninput: move |e| course_title.set(e.value())
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "Category" }
                            select {
                                class: "input",
                                value: "{course_category}",
                                oninput: move |e| course_category.set(e.value()),
                                option { value: "CivicLiteracy", "Civic Literacy" }
                                option { value: "GovernanceRoles", "Governance Roles" }
                                option { value: "TechnicalSkills", "Technical Skills" }
                                option { value: "TradeQualifications", "Trade Qualifications" }
                                option { value: "ModerationJury", "Moderation & Jury" }
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "Description" }
                            textarea {
                                class: "input",
                                style: "min-height: 80px;",
                                placeholder: "Brief course description...",
                                value: "{course_description}",
                                oninput: move |e| course_description.set(e.value())
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "Content (Markdown)" }
                            textarea {
                                class: "input font-mono text-sm",
                                style: "min-height: 200px;",
                                placeholder: "# Lesson 1\n\nYour course content here...",
                                value: "{course_content}",
                                oninput: move |e| course_content.set(e.value())
                            }
                        }
                        button { class: "btn btn-primary", onclick: on_create_course, "Create Course" }
                    }
                }
            }

            // Tab Content
            if active_tab() == "courses" {
                CoursesList {
                    on_view_course: move |node| viewing_course.set(Some(node))
                }
            } else if active_tab() == "exams" {
                ExamsList { on_take_exam: start_exam }
            } else {
                CertificationsList {}
            }
            
            // Exam Taking Modal
            if app_state.active_exam.read().is_some() {
                ExamTakingModal { 
                    current_question_idx: current_question_idx,
                    on_close: close_exam,
                    on_submit: submit_exam,
                }
            }
            
            // Exam Result Modal
            if app_state.exam_result.read().is_some() {
                ExamResultModal {
                    on_close: close_exam,
                }
            }
        }
    }
}

#[component]
fn CoursesList(on_view_course: EventHandler<crate::backend::dag::DagNode>) -> Element {
    let app_state = use_context::<AppState>();
    let courses = app_state.courses.read();
    
    rsx! {
        div { class: "grid gap-4",
            if courses.is_empty() {
                div { class: "empty-state py-12",
                    div { class: "empty-state-icon", "📚" }
                    p { class: "empty-state-title", "No courses available" }
                    p { class: "empty-state-text", "Be the first to create a course!" }
                }
            } else {
                for node in courses.iter() {
                    if let DagPayload::Course(course) = &node.payload {
                        {
                            let category_str = match &course.category {
                                CourseCategory::CivicLiteracy => "Civic Literacy",
                                CourseCategory::GovernanceRoles => "Governance Roles",
                                CourseCategory::TechnicalSkills => "Technical Skills",
                                CourseCategory::TradeQualifications => "Trade Qualifications",
                                CourseCategory::ModerationJury => "Moderation & Jury",
                                CourseCategory::Custom(s) => s.as_str(),
                            };
                            let author_short = &node.author[0..8];
                            let node_clone = node.clone();
                            
                            rsx! {
                                div { 
                                    key: "{node.id}",
                                    class: "panel hover:bg-[var(--bg-elevated)] transition-colors cursor-pointer",
                                    onclick: move |_| on_view_course.call(node_clone.clone()),
                                    div { class: "flex justify-between items-start",
                                        div { class: "flex-1",
                                            div { class: "flex gap-2 items-center mb-2",
                                                span { class: "badge badge-primary", "{category_str}" }
                                            }
                                            h3 { class: "text-lg font-bold", "{course.title}" }
                                            p { class: "text-[var(--text-secondary)] text-sm mt-1", "{course.description}" }
                                            p { class: "text-xs text-[var(--text-muted)] mt-2", "By {author_short}..." }
                                        }
                                        button { class: "btn btn-secondary btn-sm", "View Course" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CourseDetailComponent(
    course_node: crate::backend::dag::DagNode,
    on_back: EventHandler<()>,
    on_take_exam: EventHandler<crate::backend::dag::DagNode>
) -> Element {
    let app_state = use_context::<AppState>();
    let exams = app_state.exams.read();
    
    let mut show_create_exam = use_signal(|| false);

    if let DagPayload::Course(course) = &course_node.payload {
        // Filter exams for this course
        let course_exams: Vec<crate::backend::dag::DagNode> = exams.iter()
            .filter(|n| {
                if let DagPayload::Exam(e) = &n.payload {
                    e.course_id.as_ref() == Some(&course_node.id)
                } else {
                    false
                }
            })
            .cloned()
            .collect();
        
        let title = course.title.clone();
        let content = course.content.clone();
        let category = format!("{:?}", course.category);
        let author_short = if course_node.author.len() > 8 {
            format!("{}...", &course_node.author[0..8])
        } else {
            course_node.author.clone()
        };

        return rsx! {
            div { class: "page-container py-8 animate-fade-in",
                // Header
                div { class: "flex items-center gap-4 mb-6",
                    button { 
                        class: "btn btn-ghost", 
                        onclick: move |_| on_back.call(()),
                        "← Back to Courses" 
                    }
                }
                
                div { class: "grid gap-6",
                    // Course Content
                    div { class: "panel",
                        div { class: "panel-header",
                            h1 { class: "text-2xl font-bold mb-2", "{title}" }
                            div { class: "flex gap-2 text-sm text-[var(--text-secondary)]",
                                span { "By {author_short}" }
                                span { "•" }
                                span { "{category}" }
                            }
                        }
                        
                        div { class: "prose prose-invert max-w-none p-4 bg-[var(--bg-secondary)] rounded-lg whitespace-pre-wrap",
                            "{content}"
                        }
                    }
                    
                    // Exams Section
                    div { class: "panel",
                        div { class: "flex justify-between items-center mb-4",
                            h2 { class: "text-xl font-bold", "Exams & Certifications" }
                            button {
                                class: "btn btn-primary btn-sm",
                                onclick: move |_| show_create_exam.set(!show_create_exam()),
                                if show_create_exam() { "Cancel" } else { "+ Add Exam" }
                            }
                        }
                        
                        if show_create_exam() {
                            CreateExamForm { 
                                course_id: course_node.id.clone(),
                                on_created: move |_| show_create_exam.set(false)
                            }
                        }
                        
                        div { class: "grid gap-3",
                            if course_exams.is_empty() {
                                p { class: "text-[var(--text-secondary)] italic", "No exams created for this course yet." }
                            } else {
                                for exam_node in course_exams {
                                    if let DagPayload::Exam(exam) = &exam_node.payload {
                                        {
                                            let node_clone = exam_node.clone();
                                            rsx! {
                                                div { 
                                                    key: "{exam_node.id}",
                                                    class: "p-4 rounded-lg bg-[var(--bg-secondary)] border border-[var(--border)] flex justify-between items-center",
                                                    div {
                                                        h3 { class: "font-semibold", "{exam.title}" }
                                                        p { class: "text-sm text-[var(--text-secondary)]", 
                                                            "{exam.questions.len()} Questions • Pass > {exam.passing_score}%" 
                                                        }
                                                        p { class: "text-xs text-[var(--accent)]", "Grants: {exam.certification_type}" }
                                                    }
                                                    button {
                                                        class: "btn btn-primary btn-sm",
                                                        onclick: move |_| on_take_exam.call(node_clone.clone()),
                                                        "Take Exam"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };
    }
    
    rsx! { div { "Error: Invalid Course Node" } }
}

#[component]
fn CreateExamForm(course_id: String, on_created: EventHandler<()>) -> Element {
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();
    
    let mut title = use_signal(|| String::new());
    let mut passing_score = use_signal(|| 70);
    let mut cert_type = use_signal(|| String::new());
    
    // List of (Question, Options, CorrectIndex)
    let mut questions = use_signal(|| vec![
        (String::new(), vec![String::new(), String::new()], 0)
    ]);
    
    let submit = move |_| {
        let qs: Vec<(String, Vec<String>, usize)> = questions.read().clone();
        
        // Basic validation
        if title().is_empty() || cert_type().is_empty() {
            return;
        }
        for (q, opts, _) in &qs {
            if q.is_empty() || opts.iter().any(|o| o.is_empty()) {
                return;
            }
        }
        
        let _ = cmd_tx.send(AppCmd::CreateExam {
            title: title(),
            course_id: Some(course_id.clone()),
            questions: qs,
            passing_score: *passing_score.read() as u8,
            certification_type: cert_type(),
        });
        
        // Refresh exams
        let _ = cmd_tx.send(AppCmd::FetchExams);
        on_created.call(());
    };
    
    let qs_view = questions.read().clone();
    
    let questions_rendered = qs_view.into_iter().enumerate().map(|(idx, (q, opts, correct))| {
        rsx! {
            div { key: "{idx}", class: "p-3 border border-[var(--border)] rounded bg-[var(--bg-primary)]",
                div { class: "flex justify-between mb-2",
                    label { "Question {idx + 1}" }
                    if idx > 0 {
                        button { 
                            class: "text-red-500 text-xs",
                            onclick: move |_| { questions.write().remove(idx); },
                            "Remove"
                        }
                    }
                }
                input { 
                    class: "input mb-2", 
                    placeholder: "Enter question text...",
                    value: "{q}",
                    oninput: move |e| { questions.write()[idx].0 = e.value(); }
                }
                
                div { class: "pl-4 border-l-2 border-[var(--border)]",
                    label { class: "text-xs text-[var(--text-secondary)]", "Options (Select correct answer)" }
                    {
                        opts.into_iter().enumerate().map(|(opt_idx, opt_val)| {
                            let is_checked = correct == opt_idx;
                            rsx! {
                                div { key: "{opt_idx}", class: "flex items-center gap-2 mt-1",
                                    input {
                                        "type": "radio",
                                        name: "q-{idx}",
                                        checked: is_checked,
                                        onchange: move |_| { questions.write()[idx].2 = opt_idx; }
                                    }
                                    input {
                                        class: "input py-1 text-sm",
                                        value: "{opt_val}",
                                        placeholder: "Option {opt_idx + 1}",
                                        oninput: move |e| { questions.write()[idx].1[opt_idx] = e.value(); }
                                    }
                                }
                            }
                        })
                    }
                }
            }
        }
    });

    rsx! {
        div { class: "p-4 bg-[var(--bg-secondary)] rounded-lg mb-4 border border-[var(--border)]",
            h3 { class: "font-bold mb-4", "New Exam Details" }
            
            div { class: "grid gap-3 mb-4",
                div { class: "form-group",
                    label { "Exam Title" }
                    input { class: "input", value: "{title}", oninput: move |e| title.set(e.value()) }
                }
                div { class: "grid grid-cols-2 gap-4",
                    div { class: "form-group",
                        label { "Certification Name (e.g. 'Civic L1')" }
                        input { class: "input", value: "{cert_type}", oninput: move |e| cert_type.set(e.value()) }
                    }
                     div { class: "form-group",
                        label { "Passing Score (%)" }
                        input { 
                            class: "input", 
                            "type": "number", 
                            "min": "0", "max": "100",
                            value: "{passing_score}", 
                            oninput: move |e| if let Ok(n) = e.value().parse::<i32>() { passing_score.set(n) } 
                        }
                    }
                }
            }
            
            h4 { class: "font-bold mb-2", "Questions" }
            div { class: "space-y-4 mb-4",
                {questions_rendered}
                
                button {
                    class: "btn btn-secondary btn-sm w-full",
                    onclick: move |_| {
                        questions.write().push((String::new(), vec![String::new(), String::new()], 0));
                    },
                    "+ Add Question"
                }
            }
            
            div { class: "flex justify-end gap-2",
                button { 
                    class: "btn btn-primary",
                    onclick: submit,
                    "Save & Publish Exam"
                }
            }
        }
    }
}

#[component]
fn ExamsList(on_take_exam: EventHandler<crate::backend::dag::DagNode>) -> Element {
    let app_state = use_context::<AppState>();
    let exams = app_state.exams.read();
    
    rsx! {
        div { class: "grid gap-4",
            if exams.is_empty() {
                div { class: "empty-state py-12",
                    div { class: "empty-state-icon", "📝" }
                    p { class: "empty-state-title", "No exams available" }
                    p { class: "empty-state-text", "Exams can be created by course authors." }
                }
            } else {
                for node in exams.iter() {
                    if let DagPayload::Exam(exam) = &node.payload {
                        {
                            let node_clone = node.clone();
                            let author_short = &node.author[0..8.min(node.author.len())];
                            let question_count = exam.questions.len();
                            let title = exam.title.clone();
                            let passing_score = exam.passing_score;
                            let certification_type = exam.certification_type.clone();
                            
                            rsx! {
                                div { 
                                    key: "{node.id}",
                                    class: "panel",
                                    div { class: "flex justify-between items-center",
                                        div {
                                            h3 { class: "font-bold", "{title}" }
                                            p { class: "text-sm text-[var(--text-secondary)]", 
                                                "{question_count} questions • {passing_score}% to pass" 
                                            }
                                            p { class: "text-xs text-[var(--text-muted)]", 
                                                "Grants: {certification_type}" 
                                            }
                                        }
                                        button { 
                                            class: "btn btn-primary btn-sm", 
                                            onclick: move |_| on_take_exam.call(node_clone.clone()),
                                            "Take Exam" 
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CertificationsList() -> Element {
    let app_state = use_context::<AppState>();
    let certifications = app_state.certifications.read();
    
    rsx! {
        div { class: "grid gap-4",
            if certifications.is_empty() {
                div { class: "empty-state py-12",
                    div { class: "empty-state-icon", "🏆" }
                    p { class: "empty-state-title", "No certifications yet" }
                    p { class: "empty-state-text", "Complete courses and pass exams to earn certifications." }
                }
            } else {
                for node in certifications.iter() {
                    if let DagPayload::Certification(cert) = &node.payload {
                        {
                            let issued_date = cert.issued_at.format("%Y-%m-%d").to_string();
                            rsx! {
                                div { 
                                    key: "{node.id}",
                                    class: "panel bg-gradient-to-r from-[var(--primary)]/10 to-[var(--accent)]/10 border-[var(--primary)]/30",
                                    div { class: "flex items-center gap-4",
                                        div { class: "text-4xl", "🏆" }
                                        div {
                                            h3 { class: "font-bold text-lg", "{cert.certification_type}" }
                                            p { class: "text-sm text-[var(--text-secondary)]", 
                                                "Issued: {issued_date}" 
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// Exam Taking Modal - Shows questions and allows answering
#[component]
fn ExamTakingModal(
    current_question_idx: Signal<usize>,
    on_close: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let mut app_state = use_context::<AppState>();
    let active_exam = app_state.active_exam.read();
    let exam_answers = app_state.exam_answers.read();
    
    if let Some(exam_node) = active_exam.as_ref() {
        if let DagPayload::Exam(exam) = &exam_node.payload {
            let question_count = exam.questions.len();
            let current_idx = current_question_idx();
            
            if current_idx >= question_count {
                return rsx! { div {} };
            }
            
            let current_question = &exam.questions[current_idx];
            let selected = exam_answers.get(current_idx).copied().flatten();
            
            // Check if all questions answered
            let all_answered = exam_answers.iter().all(|a| a.is_some());
            let is_last = current_idx == question_count - 1;
            let is_first = current_idx == 0;
            
            let question_text = current_question.question.clone();
            let options = current_question.options.clone();
            let exam_title = exam.title.clone();
            
            return rsx! {
                // Modal overlay
                div { 
                    class: "fixed inset-0 bg-black/60 flex items-center justify-center z-50 animate-fade-in",
                    onclick: move |e| e.stop_propagation(),
                    
                    div { 
                        class: "panel max-w-2xl w-full mx-4 max-h-[90vh] overflow-y-auto",
                        onclick: move |e| e.stop_propagation(),
                        
                        // Header
                        div { class: "flex justify-between items-center mb-6",
                            div {
                                h2 { class: "text-xl font-bold", "{exam_title}" }
                                p { class: "text-sm text-[var(--text-secondary)]", 
                                    "Question {current_idx + 1} of {question_count}" 
                                }
                            }
                            button { 
                                class: "btn btn-secondary btn-sm",
                                onclick: move |_| on_close.call(()),
                                "✕ Close"
                            }
                        }
                        
                        // Progress bar
                        div { class: "w-full bg-[var(--bg-secondary)] rounded-full h-2 mb-6",
                            div { 
                                class: "bg-[var(--primary)] h-2 rounded-full transition-all",
                                style: "width: {((current_idx + 1) as f32 / question_count as f32 * 100.0)}%"
                            }
                        }
                        
                        // Question
                        div { class: "mb-6",
                            h3 { class: "text-lg font-semibold mb-4", "{question_text}" }
                            
                            // Answer options
                            div { class: "space-y-3",
                                for (idx, option) in options.iter().enumerate() {
                                    {
                                        let is_selected = selected == Some(idx);
                                        let option_text = option.clone();
                                        let option_idx = idx;
                                        
                                        rsx! {
                                            button {
                                                key: "{idx}",
                                                class: if is_selected { 
                                                    "w-full text-left p-4 rounded-lg border-2 border-[var(--primary)] bg-[var(--primary)]/10 transition-all" 
                                                } else { 
                                                    "w-full text-left p-4 rounded-lg border-2 border-[var(--border)] hover:border-[var(--primary)]/50 transition-all" 
                                                },
                                                onclick: move |_| {
                                                    let mut answers = app_state.exam_answers.write();
                                                    if current_idx < answers.len() {
                                                        answers[current_idx] = Some(option_idx);
                                                    }
                                                },
                                                div { class: "flex items-center gap-3",
                                                    div { 
                                                        class: if is_selected { 
                                                            "w-5 h-5 rounded-full border-2 border-[var(--primary)] bg-[var(--primary)] flex items-center justify-center"
                                                        } else {
                                                            "w-5 h-5 rounded-full border-2 border-[var(--text-muted)]"
                                                        },
                                                        if is_selected {
                                                            div { class: "w-2 h-2 bg-white rounded-full" }
                                                        }
                                                    }
                                                    span { "{option_text}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Navigation buttons
                        div { class: "flex justify-between gap-4",
                            if !is_first {
                                button { 
                                    class: "btn btn-secondary",
                                    onclick: move |_| {
                                        if current_question_idx() > 0 {
                                            current_question_idx.set(current_question_idx() - 1);
                                        }
                                    },
                                    "← Previous"
                                }
                            } else {
                                div {}
                            }
                            
                            if is_last {
                                button { 
                                    class: "btn btn-primary",
                                    disabled: !all_answered,
                                    onclick: move |_| on_submit.call(()),
                                    if all_answered {
                                        "Submit Exam ✓"
                                    } else {
                                        "Answer all questions"
                                    }
                                }
                            } else {
                                button { 
                                    class: "btn btn-primary",
                                    onclick: move |_| {
                                        if current_question_idx() < question_count - 1 {
                                            current_question_idx.set(current_question_idx() + 1);
                                        }
                                    },
                                    "Next →"
                                }
                            }
                        }
                    }
                }
            };
        }
    }
    
    rsx! { div {} }
}

// Exam Result Modal - Shows score after submission
#[component]
fn ExamResultModal(on_close: EventHandler<()>) -> Element {
    let app_state = use_context::<AppState>();
    let exam_result = app_state.exam_result.read();
    
    if let Some((_exam_id, score, passed)) = exam_result.as_ref() {
        let score_val = *score;
        let passed_val = *passed;
        
        return rsx! {
            // Modal overlay
            div { 
                class: "fixed inset-0 bg-black/60 flex items-center justify-center z-50 animate-fade-in",
                onclick: move |e| e.stop_propagation(),
                
                div { 
                    class: "panel max-w-md w-full mx-4 text-center",
                    onclick: move |e| e.stop_propagation(),
                    
                    // Result icon
                    div { 
                        class: "text-6xl mb-4",
                        if passed_val { "🎉" } else { "📚" }
                    }
                    
                    // Result message
                    h2 { 
                        class: if passed_val { 
                            "text-2xl font-bold text-[var(--success)] mb-2" 
                        } else { 
                            "text-2xl font-bold text-[var(--warning)] mb-2" 
                        },
                        if passed_val { "Congratulations!" } else { "Keep Learning!" }
                    }
                    
                    p { class: "text-[var(--text-secondary)] mb-6",
                        if passed_val { 
                            "You passed the exam!" 
                        } else { 
                            "You didn't pass this time, but you can try again." 
                        }
                    }
                    
                    // Score display
                    div { 
                        class: "mb-6 p-6 rounded-xl bg-[var(--bg-secondary)]",
                        div { class: "text-4xl font-bold mb-2", "{score_val}%" }
                        div { class: "text-sm text-[var(--text-secondary)]", "Your Score" }
                    }
                    
                    // Close button
                    button { 
                        class: "btn btn-primary w-full",
                        onclick: move |_| on_close.call(()),
                        "Continue"
                    }
                }
            }
        };
    }
    
    rsx! { div {} }
}
