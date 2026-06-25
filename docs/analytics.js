// Determine current page
function getPage() {
    return 'landing';
}

// Initialize data layer
window.dataLayer = window.dataLayer || [];

const page = getPage();

const uiInteractionMap = {
  // ── Header / Nav ──
  click_zellig_logo:            { element_type: "link",   element_name: "zellig_logo",        interaction_type: "click", element_location: "header", page_name: page },
  click_features:               { element_type: "link",   element_name: "features",           interaction_type: "click", element_location: "header", page_name: page },
  click_backends:               { element_type: "link",   element_name: "backends",           interaction_type: "click", element_location: "header", page_name: page },
  click_modes:                  { element_type: "link",   element_name: "modes",              interaction_type: "click", element_location: "header", page_name: page },
  click_roadmaps:               { element_type: "link",   element_name: "roadmaps",           interaction_type: "click", element_location: "header", page_name: page },
  click_faq:                    { element_type: "link",   element_name: "faq",                interaction_type: "click", element_location: "header", page_name: page },
  click_install:                { element_type: "link",   element_name: "install",            interaction_type: "click", element_location: "header", page_name: page },
  click_theme_toggle:           { element_type: "button", element_name: "theme_toggle",       interaction_type: "click", element_location: "header", page_name: page },
  click_star_github:            { element_type: "link",   element_name: "star_github",        interaction_type: "click", element_location: "header", page_name: page },
  click_install_copy:           { element_type: "button", element_name: "install_copy",       interaction_type: "click", element_location: "hero",   page_name: page },
  click_text_toggle:            { element_type: "button", element_name: "text_toggle",        interaction_type: "click", element_location: "demo",   page_name: page },
  click_file_toggle:            { element_type: "button", element_name: "file_toggle",        interaction_type: "click", element_location: "demo",   page_name: page },
  click_directory_toggle:       { element_type: "button", element_name: "directory_toggle",   interaction_type: "click", element_location: "demo",   page_name: page },
  click_interactive_toggle:     { element_type: "button", element_name: "interactive_toggle", interaction_type: "click", element_location: "demo",   page_name: page },

  // ── FAQ panels ──
  click_offline_backend:        { element_type: "pannel",  element_name: "offline_backend",     interaction_type: "click", element_location: "faq_section", page_name: page },
  click_handle_translations:    { element_type: "pannel",  element_name: "handle_translations", interaction_type: "click", element_location: "faq_section", page_name: page },
  click_custom_backend:         { element_type: "pannel",  element_name: "custom_backend",      interaction_type: "click", element_location: "faq_section", page_name: page },
  click_production_ready:       { element_type: "pannel",  element_name: "production_ready",    interaction_type: "click", element_location: "faq_section", page_name: page },
  click_tools_differences:      { element_type: "pannel",  element_name: "tools_differences",   interaction_type: "click", element_location: "faq_section", page_name: page },
  click_docker_run:             { element_type: "pannel",  element_name: "docker_run",          interaction_type: "click", element_location: "faq_section", page_name: page },
  click_glossaries:             { element_type: "pannel",  element_name: "glossaries",          interaction_type: "click", element_location: "faq_section", page_name: page },

  // ── Roadmap section ──
  click_view_on_github:         { element_type: "link",   element_name: "view_on_github",     interaction_type: "click", element_location: "roadmap_section", page_name: page },
  click_read_the_docs:          { element_type: "link",   element_name: "read_the_docs",      interaction_type: "click", element_location: "roadmap_section", page_name: page },
  click_open_pr:                { element_type: "link",   element_name: "open_pr",            interaction_type: "click", element_location: "roadmap_section", page_name: page },
  click_feature_request:        { element_type: "link",   element_name: "feature_request",     interaction_type: "click", element_location: "roadmap_section", page_name: page },
  click_bug_report:             { element_type: "link",   element_name: "bug_report",         interaction_type: "click", element_location: "roadmap_section", page_name: page },

  // ── Footer ──
  click_github:                 { element_type: "link",   element_name: "github",             interaction_type: "click", element_location: "footer", page_name: page },
  click_releases:               { element_type: "link",   element_name: "releases",           interaction_type: "click", element_location: "footer", page_name: page },
  click_changelog:              { element_type: "link",   element_name: "changelog",          interaction_type: "click", element_location: "footer", page_name: page },
  click_license:                { element_type: "link",   element_name: "license",            interaction_type: "click", element_location: "footer", page_name: page },
  click_discussions:            { element_type: "link",   element_name: "discussions",        interaction_type: "click", element_location: "footer", page_name: page },
  click_sponsor:                { element_type: "link",   element_name: "sponsor",            interaction_type: "click", element_location: "footer", page_name: page },
  click_contributing:           { element_type: "link",   element_name: "contributing",       interaction_type: "click", element_location: "footer", page_name: page },
  click_kodaskills_copyright:   { element_type: "link",   element_name: "kodaskills_copyright", interaction_type: "click", element_location: "footer", page_name: page },
  click_crates_io:              { element_type: "link",   element_name: "crates_io",          interaction_type: "click", element_location: "footer", page_name: page },
  click_getting_started:        { element_type: "link",   element_name: "getting_started",   interaction_type: "click", element_location: "footer", page_name: page },
  click_cli_reference:          { element_type: "link",   element_name: "cli_reference",     interaction_type: "click", element_location: "footer", page_name: page },
  click_backends_footer:        { element_type: "link",   element_name: "backends_footer",   interaction_type: "click", element_location: "footer", page_name: page },
  click_recipes:                { element_type: "link",   element_name: "recipes",          interaction_type: "click", element_location: "footer", page_name: page },

  // ── Section views ──
  features_section:             { element_type: "section", element_name: "features_section",  interaction_type: "view", element_location: "body", page_name: page },
  formats_section:              { element_type: "section", element_name: "formats_section",   interaction_type: "view", element_location: "body", page_name: page },
  backends_section:             { element_type: "section", element_name: "backends_section",  interaction_type: "view", element_location: "body", page_name: page },
  modes_section:                { element_type: "section", element_name: "modes_section",     interaction_type: "view", element_location: "body", page_name: page },
  benchmarks_section:           { element_type: "section", element_name: "benchmarks_section", interaction_type: "view", element_location: "body", page_name: page },
  roadmap_section:              { element_type: "section", element_name: "roadmap_section",   interaction_type: "view", element_location: "body", page_name: page },
  community_section:            { element_type: "section", element_name: "community_section", interaction_type: "view", element_location: "body", page_name: page },
  faq_section:                  { element_type: "section", element_name: "faq_section",       interaction_type: "view", element_location: "body", page_name: page },
};

function pushUiInteraction(key) {
  const params = uiInteractionMap[key];
  if (!params) return;
  window.dataLayer.push({
    event: 'ui_interaction',
    ...params
  });
}

function getThemeState() {
  const theme = document.documentElement.getAttribute('data-theme');
  if (theme === 'dark') return 'dark';
  if (theme === 'light') return 'light';
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
  return prefersDark ? 'dark' : 'light';
}

function pushSystemEvent(systemAction, systemState) {
  window.dataLayer.push({
    event: 'system_event',
    system_action: systemAction,
    page_name: page,
    system_state: systemState
  });
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
  const currentTheme = getThemeState();
  pushSystemEvent('page_load', currentTheme);

  // Click tracking
  document.addEventListener('click', (e) => {
    const trackedEl = e.target.closest('[data-analytics]');
    if (!trackedEl) return;
    pushUiInteraction(trackedEl.getAttribute('data-analytics'));
  });

  // Section view tracking
  const viewedSections = new Set();
  const observer = new IntersectionObserver((entries) => {
    entries.forEach((entry) => {
      if (!entry.isIntersecting) return;
      const key = entry.target.dataset.analyticsSection;
      if (!key || viewedSections.has(key)) return;
      viewedSections.add(key);
      pushUiInteraction(key);
    });
  }, { threshold: 0.3 });

  document.querySelectorAll('[data-analytics-section]').forEach((el) => observer.observe(el));
});
