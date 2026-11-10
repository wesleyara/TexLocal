/** Árvore de arquivos a partir de caminhos relativos (com `/`). */

export interface TreeNode {
  name: string
  path: string
  dir: boolean
  children: TreeNode[]
}

export function buildTree(paths: string[]): TreeNode[] {
  const root: TreeNode = { name: '', path: '', dir: true, children: [] }
  for (const p of paths) {
    const parts = p.split('/')
    let node = root
    parts.forEach((name, i) => {
      const path = parts.slice(0, i + 1).join('/')
      const isDir = i < parts.length - 1
      let child = node.children.find((c) => c.name === name && c.dir === isDir)
      if (!child) {
        child = { name, path, dir: isDir, children: [] }
        node.children.push(child)
      }
      node = child
    })
  }
  const sort = (n: TreeNode) => {
    n.children.sort(
      (a, b) =>
        Number(b.dir) - Number(a.dir) || a.name.toLowerCase().localeCompare(b.name.toLowerCase()),
    )
    n.children.forEach(sort)
  }
  sort(root)
  return root.children
}

export interface TreeRow {
  name: string
  path: string
  dir: boolean
  depth: number
}

/** Linhas visíveis, respeitando pastas recolhidas. */
export function flattenTree(nodes: TreeNode[], collapsed: Set<string>, depth = 0): TreeRow[] {
  return nodes.flatMap((n) => [
    { name: n.name, path: n.path, dir: n.dir, depth },
    ...(n.dir && !collapsed.has(n.path) ? flattenTree(n.children, collapsed, depth + 1) : []),
  ])
}
