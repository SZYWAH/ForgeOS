import { Link } from "react-router-dom";

export function NotFoundPage() {
  return (
    <section className="panel not-found">
      <p className="eyebrow">404</p>
      <h1>页面不存在</h1>
      <p>这个 ForgeOS 页面不存在。</p>
      <Link to="/">返回仪表盘</Link>
    </section>
  );
}
