"use strict";

const TOKEN_KEY = "forum-engine-session";

const state = {
  token: localStorage.getItem(TOKEN_KEY),
  user: null,
  categories: [],
  topics: [],
  selectedCategory: "all",
  query: "",
  authMode: "login",
  currentTopicId: null,
};

const elements = Object.fromEntries(
  [
    "notice", "home-view", "topic-view", "category-list", "all-topic-count",
    "topic-list", "topic-loading", "topic-empty", "topic-list-title", "topic-search",
    "login-button", "register-button", "user-button", "user-menu", "user-summary",
    "logout-button", "new-topic-button", "new-category-button", "categories-button",
    "back-button", "topic-category", "topic-title", "topic-meta", "post-list",
    "reply-form", "reply-body", "reply-hint", "auth-dialog", "auth-form",
    "auth-eyebrow", "auth-title", "auth-login-label", "auth-login", "auth-username",
    "auth-email", "auth-password", "auth-submit", "auth-switch", "auth-switch-copy",
    "auth-error", "topic-dialog", "topic-form", "topic-category-input",
    "topic-title-input", "topic-body-input", "topic-error", "category-dialog",
    "category-form", "category-name", "category-slug", "category-error", "toast",
  ].map((id) => [id, document.getElementById(id)])
);

const errorMessages = {
  "invalid JSON": "请求内容无效。",
  "invalid username": "用户名需要使用有效格式。",
  "invalid email": "请输入有效的邮箱地址。",
  "password must contain 12 to 256 bytes": "密码长度需要在 12 到 256 字节之间。",
  "username or email already exists": "用户名或邮箱已经被注册。",
  "invalid credentials": "用户名、邮箱或密码不正确。",
  "authentication required": "请先登录。",
  "administrator required": "只有管理员可以执行此操作。",
  "invalid category name": "分类名称长度需要在 2 到 80 个字符之间。",
  "invalid category slug": "英文标识只能包含小写字母、数字和连字符。",
  "category slug already exists": "这个分类英文标识已经存在。",
  "invalid title": "标题长度需要在 3 到 200 个字符之间。",
  "invalid body": "正文不能为空。",
  "category not found": "分类不存在。",
  "topic not found": "主题不存在。",
};

async function api(path, options = {}) {
  const headers = new Headers(options.headers || {});
  if (options.body && !headers.has("Content-Type")) headers.set("Content-Type", "application/json");
  if (state.token) headers.set("Authorization", `Bearer ${state.token}`);

  let response;
  try {
    response = await fetch(path, { ...options, headers });
  } catch {
    throw new Error("无法连接服务器，请检查网络后重试。");
  }

  let data = null;
  if (response.status !== 204) {
    const contentType = response.headers.get("Content-Type") || "";
    data = contentType.includes("application/json") ? await response.json() : null;
  }
  if (!response.ok) {
    if (response.status === 401 && path === "/api/v1/me") clearSession();
    const message = data?.error ? errorMessages[data.error] || data.error : `请求失败（${response.status}）`;
    throw new Error(message);
  }
  return data;
}

function clearSession() {
  state.token = null;
  state.user = null;
  localStorage.removeItem(TOKEN_KEY);
  renderAuth();
}

function saveSession(session) {
  state.token = session.token;
  state.user = session.user;
  localStorage.setItem(TOKEN_KEY, session.token);
  renderAuth();
}

function renderAuth() {
  const loggedIn = Boolean(state.user);
  elements.loginButton.classList.toggle("hidden", loggedIn);
  elements.registerButton.classList.toggle("hidden", loggedIn);
  elements.userButton.classList.toggle("hidden", !loggedIn);
  elements.newCategoryButton.classList.toggle("hidden", state.user?.role !== "admin");
  elements.replyBody.disabled = !loggedIn;
  elements.replyHint.textContent = loggedIn ? `正在以 ${state.user.username} 的身份回复。` : "登录后可以参与讨论。";

  if (loggedIn) {
    elements.userButton.textContent = state.user.username.slice(0, 1);
    elements.userButton.setAttribute("aria-label", `${state.user.username} 的账户菜单`);
    elements.userSummary.replaceChildren(
      textElement("strong", state.user.username),
      textElement("span", roleLabel(state.user.role))
    );
  } else {
    elements.userMenu.classList.add("hidden");
  }
}

function roleLabel(role) {
  return { admin: "管理员", moderator: "版主", member: "成员" }[role] || role;
}

function textElement(tag, text, className) {
  const node = document.createElement(tag);
  node.textContent = text;
  if (className) node.className = className;
  return node;
}

function categoryName(id) {
  return state.categories.find((category) => category.id === id)?.name || "未分类";
}

function formatDate(value) {
  const date = new Date(value.endsWith("Z") ? value : `${value.replace(" ", "T")}Z`);
  if (Number.isNaN(date.getTime())) return value;
  const seconds = Math.round((date.getTime() - Date.now()) / 1000);
  const ranges = [
    [31536000, "year"], [2592000, "month"], [86400, "day"],
    [3600, "hour"], [60, "minute"], [1, "second"],
  ];
  const formatter = new Intl.RelativeTimeFormat("zh-CN", { numeric: "auto" });
  for (const [size, unit] of ranges) {
    if (Math.abs(seconds) >= size || unit === "second") return formatter.format(Math.round(seconds / size), unit);
  }
  return value;
}

function renderCategories() {
  elements.categoryList.replaceChildren();
  elements.allTopicCount.textContent = String(state.topics.length);
  for (const [index, category] of state.categories.entries()) {
    const count = state.topics.filter((topic) => topic.category_id === category.id).length;
    const button = document.createElement("button");
    button.type = "button";
    button.className = "category-item";
    button.dataset.category = category.id;
    const label = document.createElement("span");
    label.className = "category-label";
    const dot = document.createElement("span");
    dot.className = "category-dot";
    dot.dataset.tone = String((index % 4) + 1);
    label.append(dot, document.createTextNode(category.name));
    button.append(label, textElement("span", String(count)));
    elements.categoryList.append(button);
  }
  document.querySelectorAll(".category-item").forEach((button) => {
    button.classList.toggle("active", button.dataset.category === state.selectedCategory);
  });

  elements.topicCategoryInput.replaceChildren();
  if (state.categories.length === 0) {
    const option = new Option("请先由管理员创建分类", "");
    option.disabled = true;
    option.selected = true;
    elements.topicCategoryInput.add(option);
  } else {
    for (const category of state.categories) elements.topicCategoryInput.add(new Option(category.name, category.id));
  }
}

function filteredTopics() {
  const query = state.query.trim().toLocaleLowerCase("zh-CN");
  return state.topics.filter((topic) => {
    const categoryMatches = state.selectedCategory === "all" || topic.category_id === state.selectedCategory;
    const queryMatches = !query || topic.title.toLocaleLowerCase("zh-CN").includes(query) || topic.author_username.toLocaleLowerCase("zh-CN").includes(query);
    return categoryMatches && queryMatches;
  });
}

function renderTopics() {
  const topics = filteredTopics();
  elements.topicLoading.classList.add("hidden");
  elements.topicList.classList.toggle("hidden", topics.length === 0);
  elements.topicEmpty.classList.toggle("hidden", topics.length !== 0);
  elements.topicList.replaceChildren();

  const selected = state.categories.find((item) => item.id === state.selectedCategory);
  elements.topicListTitle.textContent = selected?.name || "最新主题";

  for (const topic of topics) {
    const row = document.createElement("article");
    row.className = "topic-row";
    row.tabIndex = 0;
    row.setAttribute("role", "link");
    row.dataset.topicId = topic.id;
    const main = document.createElement("div");
    main.className = "topic-summary";
    main.append(textElement("h3", topic.title));
    const meta = document.createElement("div");
    meta.className = "topic-row-meta";
    const avatar = textElement("span", topic.author_username.slice(0, 1), "mini-avatar");
    avatar.setAttribute("aria-hidden", "true");
    meta.append(
      avatar,
      textElement("span", topic.author_username, "topic-author"),
      textElement("span", categoryName(topic.category_id), "category-pill")
    );
    main.append(meta);
    const replies = document.createElement("div");
    replies.className = "topic-stat reply-count";
    replies.append(
      textElement("strong", String(Math.max(0, topic.post_count - 1))),
      textElement("span", "回复")
    );
    const activity = document.createElement("div");
    activity.className = "topic-stat activity-time";
    activity.append(
      textElement("strong", formatDate(topic.updated_at)),
      textElement("span", "活动")
    );
    row.append(main, replies, activity);
    elements.topicList.append(row);
  }
}

async function loadHome() {
  elements.topicLoading.classList.remove("hidden");
  elements.topicList.classList.add("hidden");
  elements.topicEmpty.classList.add("hidden");
  try {
    const [categories, topics] = await Promise.all([
      api("/api/v1/categories"),
      api("/api/v1/topics"),
    ]);
    state.categories = categories.items;
    state.topics = topics.items;
    renderCategories();
    renderTopics();
    elements.notice.classList.add("hidden");
  } catch (error) {
    showNotice(error.message);
    elements.topicLoading.classList.add("hidden");
  }
}

function showHome() {
  state.currentTopicId = null;
  elements.homeView.classList.remove("hidden");
  elements.topicView.classList.add("hidden");
  window.scrollTo({ top: 0, behavior: "smooth" });
}

async function showTopic(id) {
  state.currentTopicId = id;
  elements.homeView.classList.add("hidden");
  elements.topicView.classList.remove("hidden");
  elements.topicTitle.textContent = "正在加载…";
  elements.topicMeta.textContent = "";
  elements.postList.replaceChildren();
  window.scrollTo({ top: 0, behavior: "smooth" });
  try {
    const detail = await api(`/api/v1/topics/${encodeURIComponent(id)}`);
    elements.topicTitle.textContent = detail.topic.title;
    elements.topicCategory.textContent = categoryName(detail.topic.category_id);
    elements.topicMeta.textContent = `${detail.topic.author_username} 发起 · ${formatDate(detail.topic.created_at)} · ${detail.posts.length} 条内容`;
    renderPosts(detail.posts);
    document.title = `${detail.topic.title} · 社区论坛`;
  } catch (error) {
    showNotice(error.message);
    location.hash = "#/";
  }
}

function renderPosts(posts) {
  elements.postList.replaceChildren();
  for (const post of posts) {
    const article = document.createElement("article");
    article.className = "post";
    const author = document.createElement("aside");
    author.className = "post-author";
    author.append(
      textElement("div", post.author_username.slice(0, 1), "avatar"),
      textElement("strong", post.author_username),
      textElement("span", `第 ${post.position} 楼`)
    );
    const content = document.createElement("div");
    content.className = "post-content";
    const header = document.createElement("div");
    header.className = "post-content-header";
    header.append(textElement("span", formatDate(post.created_at)), textElement("span", `#${post.position}`));
    content.append(header, textElement("div", post.body, "post-body"));
    article.append(author, content);
    elements.postList.append(article);
  }
}

function showNotice(message) {
  elements.notice.textContent = message;
  elements.notice.classList.remove("hidden");
}

let toastTimer;
function toast(message) {
  clearTimeout(toastTimer);
  elements.toast.textContent = message;
  elements.toast.classList.remove("hidden");
  toastTimer = setTimeout(() => elements.toast.classList.add("hidden"), 2600);
}

function showFormError(element, message) {
  element.textContent = message;
  element.classList.remove("hidden");
}

function openAuth(mode) {
  state.authMode = mode;
  renderAuthDialog();
  elements.authError.classList.add("hidden");
  elements.authForm.reset();
  elements.authDialog.showModal();
}

function renderAuthDialog() {
  const register = state.authMode === "register";
  document.querySelectorAll(".register-only").forEach((item) => item.classList.toggle("hidden", !register));
  elements.authUsername.required = register;
  elements.authEmail.required = register;
  elements.authLogin.required = !register;
  elements.authLogin.classList.toggle("hidden", register);
  elements.authLoginLabel.classList.toggle("hidden", register);
  elements.authEyebrow.textContent = register ? "加入社区" : "欢迎回来";
  elements.authTitle.textContent = register ? "创建账户" : "登录";
  elements.authSubmit.textContent = register ? "注册并登录" : "登录";
  elements.authSwitchCopy.firstChild.textContent = register ? "已有账户？ " : "还没有账户？ ";
  elements.authSwitch.textContent = register ? "登录" : "注册";
  elements.authPassword.autocomplete = register ? "new-password" : "current-password";
}

async function restoreSession() {
  if (!state.token) return renderAuth();
  try {
    state.user = await api("/api/v1/me");
  } catch {
    clearSession();
  }
  renderAuth();
}

function requireLogin() {
  if (state.user) return true;
  openAuth("login");
  toast("请先登录。");
  return false;
}

function currentRoute() {
  const match = location.hash.match(/^#\/topic\/([^/]+)$/);
  return match ? { name: "topic", id: decodeURIComponent(match[1]) } : { name: "home" };
}

function route() {
  const current = currentRoute();
  if (current.name === "topic") showTopic(current.id);
  else {
    document.title = "社区论坛";
    showHome();
  }
}

elements.loginButton.addEventListener("click", () => openAuth("login"));
elements.registerButton.addEventListener("click", () => openAuth("register"));
elements.authSwitch.addEventListener("click", () => {
  state.authMode = state.authMode === "login" ? "register" : "login";
  elements.authForm.reset();
  elements.authError.classList.add("hidden");
  renderAuthDialog();
});

elements.authForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  elements.authError.classList.add("hidden");
  const submit = elements.authSubmit;
  submit.disabled = true;
  try {
    const register = state.authMode === "register";
    const body = register
      ? { username: elements.authUsername.value, email: elements.authEmail.value, password: elements.authPassword.value }
      : { login: elements.authLogin.value, password: elements.authPassword.value };
    const session = await api(register ? "/api/v1/auth/register" : "/api/v1/auth/login", {
      method: "POST",
      body: JSON.stringify(body),
    });
    saveSession(session);
    elements.authDialog.close();
    toast(register ? "账户已创建。" : "已登录。");
    await loadHome();
  } catch (error) {
    showFormError(elements.authError, error.message);
  } finally {
    submit.disabled = false;
  }
});

elements.logoutButton.addEventListener("click", async () => {
  try { await api("/api/v1/auth/logout", { method: "POST" }); } catch { /* Clear locally even if the session already expired. */ }
  clearSession();
  elements.userMenu.classList.add("hidden");
  toast("已退出登录。");
});

elements.userButton.addEventListener("click", () => elements.userMenu.classList.toggle("hidden"));
document.addEventListener("click", (event) => {
  if (!elements.userMenu.contains(event.target) && event.target !== elements.userButton) elements.userMenu.classList.add("hidden");
});

document.getElementById("category-sidebar").addEventListener("click", (event) => {
  const button = event.target.closest(".category-item");
  if (!button) return;
  state.selectedCategory = button.dataset.category;
  renderCategories();
  renderTopics();
});

elements.categoriesButton.addEventListener("click", () => {
  location.hash = "#/";
  document.getElementById("category-sidebar").scrollIntoView({ behavior: "smooth", block: "start" });
});
elements.topicSearch.addEventListener("input", () => { state.query = elements.topicSearch.value; renderTopics(); });
elements.topicList.addEventListener("click", (event) => {
  const row = event.target.closest(".topic-row");
  if (row) location.hash = `#/topic/${encodeURIComponent(row.dataset.topicId)}`;
});
elements.topicList.addEventListener("keydown", (event) => {
  if (event.key !== "Enter" && event.key !== " ") return;
  const row = event.target.closest(".topic-row");
  if (row) { event.preventDefault(); location.hash = `#/topic/${encodeURIComponent(row.dataset.topicId)}`; }
});
elements.backButton.addEventListener("click", () => { location.hash = "#/"; });

elements.newTopicButton.addEventListener("click", () => {
  if (!requireLogin()) return;
  if (state.categories.length === 0) return toast(state.user.role === "admin" ? "请先创建一个分类。" : "管理员尚未创建分类。");
  elements.topicError.classList.add("hidden");
  elements.topicForm.reset();
  elements.topicDialog.showModal();
});

elements.topicForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  elements.topicError.classList.add("hidden");
  const submit = event.submitter;
  submit.disabled = true;
  try {
    const result = await api("/api/v1/topics", {
      method: "POST",
      body: JSON.stringify({
        category_id: elements.topicCategoryInput.value,
        title: elements.topicTitleInput.value,
        body: elements.topicBodyInput.value,
      }),
    });
    elements.topicDialog.close();
    await loadHome();
    location.hash = `#/topic/${encodeURIComponent(result.id)}`;
    toast("主题已发布。");
  } catch (error) {
    showFormError(elements.topicError, error.message);
  } finally { submit.disabled = false; }
});

elements.replyForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (!requireLogin() || !state.currentTopicId) return;
  const submit = event.submitter;
  submit.disabled = true;
  try {
    await api(`/api/v1/topics/${encodeURIComponent(state.currentTopicId)}/posts`, {
      method: "POST",
      body: JSON.stringify({ body: elements.replyBody.value }),
    });
    elements.replyForm.reset();
    await showTopic(state.currentTopicId);
    toast("回复已发布。");
  } catch (error) { toast(error.message); }
  finally { submit.disabled = false; }
});

elements.newCategoryButton.addEventListener("click", () => {
  elements.categoryForm.reset();
  elements.categorySlug.dataset.edited = "false";
  elements.categoryError.classList.add("hidden");
  elements.categoryDialog.showModal();
});
elements.categoryName.addEventListener("input", () => {
  if (elements.categorySlug.dataset.edited === "true") return;
  elements.categorySlug.value = elements.categoryName.value.trim().toLowerCase()
    .replace(/[^a-z0-9\s-]/g, "").replace(/\s+/g, "-").replace(/-+/g, "-");
});
elements.categorySlug.addEventListener("input", () => { elements.categorySlug.dataset.edited = "true"; });
elements.categoryForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  elements.categoryError.classList.add("hidden");
  const submit = event.submitter;
  submit.disabled = true;
  try {
    await api("/api/v1/categories", {
      method: "POST",
      body: JSON.stringify({ name: elements.categoryName.value, slug: elements.categorySlug.value }),
    });
    elements.categoryDialog.close();
    elements.categorySlug.dataset.edited = "false";
    await loadHome();
    toast("分类已创建。");
  } catch (error) { showFormError(elements.categoryError, error.message); }
  finally { submit.disabled = false; }
});

document.querySelectorAll("[data-close-dialog]").forEach((button) => {
  button.addEventListener("click", () => button.closest("dialog").close());
});
document.querySelectorAll("dialog").forEach((dialog) => {
  dialog.addEventListener("click", (event) => {
    if (event.target === dialog) dialog.close();
  });
});

window.addEventListener("hashchange", route);

async function init() {
  await Promise.all([restoreSession(), loadHome()]);
  route();
}

init();
